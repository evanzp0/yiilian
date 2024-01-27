use chrono::Utc;
use futures::{stream::FuturesUnordered, StreamExt};
use std::{
    collections::HashSet,
    fmt::Debug,
    fs,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};
use tokio::{
    net::{lookup_host, UdpSocket},
    time::sleep,
};
use yiilian_core::{
    common::{error::Error, shutdown::ShutdownReceiver, util::random_bytes},
    net::block_list::{BlockAddr, BlockList},
};

use crate::{
    common::{
        context::Context,
        id::Id,
        ip::IPV4Consensus,
        setting::{Settings, SettingsBuilder},
        state::State,
    },
    data::body::KrpcBody,
    net::{Client, Server},
    peer::PeerManager,
    routing_table::{Persist, RoutingTable},
    service::MakeServiceRef,
    transaction::TransactionManager,
};

pub struct Dht<S> {
    ctx: Arc<Context>,
    server: Server<S>,
}

impl<S> Dht<S>
where
    S: MakeServiceRef<Context, KrpcBody, ResBody = KrpcBody>,
    S::Service: Send + 'static,
    S::Error: Debug + Send,
{
    pub fn init(
        local_addr: SocketAddr,
        service: S,
        settings: Option<Settings>,
        block_list: Option<HashSet<BlockAddr>>,
        shutdown_rx: ShutdownReceiver,
    ) -> Result<Self, Error> {
        let local_id = Id::from_ip(&local_addr.ip());

        let settings = if let Some(val) = settings {
            val
        } else {
            SettingsBuilder::new().build()
        };

        let transaction_manager = TransactionManager::new(local_addr, shutdown_rx.clone());

        let routing_table = build_routing_table(
            local_id,
            settings.block_list_max_size,
            settings.bucket_size,
            block_list,
        );

        let state = build_state(local_addr, local_id, settings.token_secret_size)?;

        let peer_manager = Mutex::new(PeerManager::new(
            settings.max_resources,
            settings.max_peers_per_resource,
        ));

        let socket = Arc::new(build_socket(local_addr)?);

        let client = Client::new(socket.clone());

        let ctx = Context::new(
            local_addr,
            settings,
            state,
            routing_table,
            peer_manager,
            transaction_manager,
            client,
        );
        let ctx = Arc::new(ctx);

        let server = Server::new(socket.clone(), service, ctx.clone());

        Ok(Dht { ctx, server })
    }

    pub async fn run_loop(&self) {
        let port = self.ctx.local_addr().port();

        // 各种周期性的 future
        // tokio::try_join! 全部完成或有一个 Err 时退出
        match tokio::try_join!(
            self.server.run_loop(),
            self.ping_persist_once(),
            self.periodic_router_ping(),
            self.periodic_buddy_ping(),
            self.periodic_find_node(),
            self.periodic_ip4_maintenance(),
            self.periodic_token_rotation(),
            self.ctx.transaction_manager().request_cleanup(self.ctx.clone()),
        ) {
            Ok(_) => (),
            Err(e) => {
               log::debug!(target: "yiilian_dht::dht::run_loop", "[{}] Quit with error: {}", port, e);
            },
        }
    }

    /// Build and send a ping to a target. Doesn't wait for a response
    /// 生成并发任务执行 ping 请求，并等待响应
    async fn ping_persist_once(&self) -> Result<(), Error> {
        let port = self.ctx.local_addr().port();
        let nodes_file = self.ctx.state().read().unwrap().nodes_file.clone();
        match fs::read_to_string(&nodes_file) {
            Ok(val) => {
                if let Ok(persist) = serde_yaml::from_str::<Persist>(&val) {
                    log::trace!(target: "yiilian_dht::dht::ping_persist_once", " [{}] Enter ping_persist_once", port);

                    for node_addr in persist.node_addrs {
                        let rst = self
                            .ctx
                            .transaction_manager()
                            .ping_no_wait(node_addr, None, self.ctx.clone())
                            .await;

                        match rst {
                            Err(e) => {
                                log::debug!(target: "yiilian_dht::dht::ping_persist_once", "[{}] ping_no_wait error: {:?}", port, e);
                            }
                            _ => (),
                        }
                        sleep(Duration::from_millis(10)).await;
                    }
                } else {
                    log::debug!(target: "yiilian_dht::dht::ping_persist_once", "[{}] Parsing file {:?} error", port, nodes_file.as_os_str());
                }
            }
            Err(e) => Err(Error::new_file(Some(e.into()), None))?,
        }

        Ok(())
    }

    /// 周期性 ping 入口 router，可以获取对方识别的我们外网 ip，并且有机会让对方将我们加入 routing table
    /// 同时，在 ping 反馈时，我们也会将对方加入 routing table
    async fn periodic_router_ping(&self) -> Result<(), Error> {
        let port = self.ctx.local_addr().port();
        loop {
            let is_join_kad = self.ctx.state().read().unwrap().is_join_kad;
            let router_ping_interval_sec = {
                if is_join_kad {
                    self.ctx.settings().router_ping_interval_secs
                } else {
                    self.ctx.settings().router_ping_if_not_join_interval_secs
                }
            };

            log::trace!(
                target: "yiilian_dht::dht::periodic_router_ping",
                "[{}] Enter periodic_router_ping, is_join_kad: {}, interval_sec: {}",
                port, is_join_kad, router_ping_interval_sec
            );

            self.ping_routers().await;

            sleep(Duration::from_secs(router_ping_interval_sec)).await;
        }
    }

    /// 按照随即顺序 ping 多个 router，直到所有的 ping 请求对方都响应了消息，或者 ping 请求在接收响应前超时
    /// FuturesUnordered类型允许 Future 以任意顺序执行
    async fn ping_routers(&self) {
        let port = self.ctx.local_addr().port();
        let mut futures = FuturesUnordered::new();
        // 入口 router
        let routers = &self.ctx.settings().routers;

        for hostname in routers {
            futures.push(self.ping_router(hostname.clone()));
        }

        while let Some(rst) = futures.next().await {
            match rst {
                Err(e) => {
                    log::debug!(target:"yiilian_dht::dht::ping_routers", "[{}] error: {:?}", port, e);
                }
                _ => (),
            }
        }
    }

    /// 将 “域名:PORT” 解析为 “IPv4:PORT” ，并向对方发送 PING 请求，并等待响应
    async fn ping_router(&self, hostname: String) -> Result<(), Error> {
        // 解析域名
        let resolve = lookup_host(&hostname).await;

        match resolve {
            Err(err) => {
                // Used to only eat the specific errors corresponding to a failure to resolve,
                // but they vary by platform and it's a pain. For now, we'll eat all host
                // resolution errors.
                Err(Error::new_net(
                    Some(err.into()),
                    Some(format!("Failed to resolve host {}", hostname)),
                    None,
                ))?
            }
            Ok(val) => {
                // 对解析出的 ip 地址，并发发送 ping 请求并处理其响应
                for socket_addr in val {
                    if socket_addr.is_ipv4() {
                        self.ctx
                            .routing_table()
                            .lock()
                            .unwrap()
                            .white_list
                            .insert(socket_addr.ip());

                        // 生成并发任务执行 ping 请求，并等待响应
                        self.ctx
                            .transaction_manager()
                            .ping_no_wait(socket_addr, None, self.ctx.clone())
                            .await?;
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// 周期性 ping 路由表中的节点
    async fn periodic_buddy_ping(&self) -> Result<(), Error> {
        let port = self.ctx.local_addr().port();
        // 每隔 10 秒做一次 ping 检查
        let ping_check_interval_secs = self.ctx.settings().ping_check_interval_secs;

        loop {
            sleep(Duration::from_secs(ping_check_interval_secs)).await; // 由于有这个 sleep，在它挂起任务时，就有机会优雅退出

            if !self.ctx.state().read().unwrap().is_join_kad {
                continue;
            }

            log::trace!(target: "yiilian_dht::dht::periodic_buddy_ping", "[{}] Enter periodic_buddy_ping", port);

            // 将需要状态的东西打包到一个块中，这样 Rust 就不会抱怨 MutexGuard 跨 .await 了
            let reverify_interval_secs = {
                let reverify_grace_period_secs = self.ctx.settings().reverify_grace_period_secs;
                let verify_grace_period_secs = self.ctx.settings().verify_grace_period_secs;

                // 将过期没再次校验的节点从 buckets 中删除
                self.ctx.routing_table().lock().unwrap().prune(
                    Duration::from_secs(reverify_grace_period_secs), // 每隔 14 分钟一次
                    Duration::from_secs(verify_grace_period_secs),   // 每隔 1 分钟一次
                    self.ctx.clone(),
                );

                // 验证的有效时间为 15 分钟
                self.ctx.settings().reverify_interval_secs
            };

            // 到了 reverify_interval_secs 再次验证时间间隔，需要将所有的 node （已验证/未验证） 都 ping 一遍
            // 超过 ping_if_older_than 时间点的节点，都需要被 ping
            let ping_if_older_than = Utc::now() - Duration::from_secs(reverify_interval_secs);

            let (unverified, verified) = {
                let unverified = self
                    .ctx
                    .routing_table()
                    .lock()
                    .unwrap()
                    .get_all_unverified();
                let verified = self.ctx.routing_table().lock().unwrap().get_all_verified();
                (unverified, verified)
            };

            // Ping everybody we haven't verified
            for node in unverified {
                // Some things in here are actually verified... don't bother them too often
                if let Some(last_verified) = node.last_verified {
                    if last_verified >= ping_if_older_than {
                        // 最后验证的时间晚于 ping_if_older_than 时间点，这次就不需要 ping 了
                        continue;
                    }
                }

                // 生成并发任务执行 ping 请求，并等待响应
                let rst = self
                    .ctx
                    .transaction_manager()
                    .ping_no_wait(node.address, Some(node.id), self.ctx.clone())
                    .await;

                match rst {
                    Err(e) => {
                        log::debug!(target:"yiilian_dht::dht::periodic_buddy_ping", "[{}] Error ping unverified: {:?}", port, e);
                    }
                    _ => (),
                }
            }

            // Reverify those who haven't been verified recently
            for node in verified {
                if let Some(last_verified) = node.last_verified {
                    if last_verified >= ping_if_older_than {
                        continue;
                    }
                }

                let rst = self
                    .ctx
                    .transaction_manager()
                    .ping_no_wait(node.address, Some(node.id), self.ctx.clone())
                    .await;

                match rst {
                    Err(e) => {
                        log::debug!(target:"yiilian_dht::dht::periodic_buddy_ping", "[{}] Error ping verified: {:?}", port, e);
                    }
                    _ => (),
                }
            }
        }
    }

    /// 周期性 find_node 一个随机生成的接近本机节点的 Node id
    async fn periodic_find_node(&self) -> Result<(), Error> {
        let port = self.ctx.local_addr().port();
        let find_node_interval_secs = self.ctx.settings().find_nodes_interval_secs; // 33 s
        loop {
            sleep(Duration::from_secs(find_node_interval_secs)).await;

            if !self.ctx.state().read().unwrap().is_join_kad {
                continue;
            }

            log::trace!(target: "yiilian_dht::dht::periodic_find_node", "[{}] Enter periodic_find_node", port);

            let (count_unverified, count_verified) =
                self.ctx.routing_table().lock().unwrap().count();

            // 如果路由表中没有 node ，则 ping 入口 router。
            // 当我们已经睡了一段时间并且失去了所有节点，这会很有帮助。
            if count_verified == 0 {
                self.ping_routers().await;
            }

            // 有足够多的未验证节点，则不需要本次 find_node 了
            let id_near_us = {
                let find_nodes_skip_count = self.ctx.settings().find_nodes_skip_count;
                if count_unverified > find_nodes_skip_count {
                    continue;
                }

                // 生成一个和本机 ID 接近的新 ID （只有后 4 个字节不同）
                self.ctx
                    .state()
                    .read()
                    .unwrap()
                    .local_id
                    .make_mutant(4)
                    .unwrap()
            };

            // 向这些附近节点中发送 find_node 本机节点的请求
            self.ctx
                .transaction_manager()
                .find_node(id_near_us, self.ctx.clone())
                .await;
        }
    }

    /// 每隔 10 秒，周期性维护 IPv4 （使用本机的最佳外网IP地址生成本机节点 ID）
    async fn periodic_ip4_maintenance(&self) -> Result<(), Error> {
        let port = self.ctx.local_addr().port();
        let ip4_maintenance_interval_sec = self.ctx.settings().ip4_maintenance_interval_sec;

        loop {
            sleep(Duration::from_secs(ip4_maintenance_interval_sec)).await;
            log::trace!(target: "yiilian_dht::dht::periodic_ip4_maintenance", "[{}] Enter periodic_ip4_maintenance", port);

            // 每隔 10 秒，将各 ip 投票数 - 1
            self.ctx.state().write().unwrap().ip4_source.decay();

            let best_ipv4 = self.ctx.state().read().unwrap().ip4_source.get_best_ipv4();
            if let Some(ip) = best_ipv4 {
                // 取出被投票数最多的外网 ipv4 地址，如果获取的投票数没超过阈值，则返回 None
                let ip = IpAddr::V4(ip);
                let local_id = self.ctx.state().read().unwrap().local_id.clone();
                // 如果本机外网 ip 地址和 本机节点 id 没有有效匹配，则生成一个新的有效匹配的本机 node id
                if !local_id
                    .is_valid_for_ip(&ip, &self.ctx.routing_table().lock().unwrap().white_list)
                {
                    let new_id = Id::from_ip(&ip);
                    self.ctx.state().write().unwrap().local_id = new_id;
                }
            }
        }
    }

    /// 定期维护 token
    async fn periodic_token_rotation(&self) -> Result<(), Error> {
        let port = self.ctx.local_addr().port();
        let token_refresh_interval_sec = self.ctx.settings().token_refresh_interval_sec;

        loop {
            log::trace!(target: "yiilian_dht::dht::periodic_token_rotation", "[{}] Enter periodic_token_rotation", port);

            sleep(Duration::from_secs(token_refresh_interval_sec)).await;
            self.rotate_token_secrets();
        }
    }

    /// 更新 token_secret
    fn rotate_token_secrets(&self) {
        let new_token_secret = random_bytes(self.ctx.settings().token_secret_size);
        let old_token_secret = self.ctx.state().read().unwrap().token_secret.clone();

        self.ctx.state().write().unwrap().old_token_secret = old_token_secret;
        self.ctx.state().write().unwrap().token_secret = new_token_secret;
    }
}

fn build_routing_table(
    local_id: Id,
    block_list_max_size: i32,
    bucket_size: usize,
    block_list: Option<HashSet<BlockAddr>>,
) -> Mutex<RoutingTable> {
    let block_list = BlockList::new(block_list_max_size, block_list);
    let routing_table = RoutingTable::new(bucket_size, block_list, local_id);

    Mutex::new(routing_table)
}

fn build_state(
    local_addr: SocketAddr,
    local_id: Id,
    token_secret_size: usize,
) -> Result<RwLock<State>, Error> {
    let port = local_addr.port();
    let token_secret = random_bytes(token_secret_size);

    let nodes_file = home::home_dir()
        .map_or(
            Err(Error::new_path(
                None,
                Some(format!("<user home> not found")),
            )),
            |v| Ok(v),
        )?
        .join(format!(".yiilian/dht/{}.txt", port));

    Ok(RwLock::new(State::new(
        local_id,
        IPV4Consensus::new(2, 10),
        token_secret,
        nodes_file,
    )))
}

fn build_socket(socket_addr: SocketAddr) -> Result<UdpSocket, Error> {
    let std_sock =
        std::net::UdpSocket::bind(socket_addr).map_err(|e| Error::new_bind(Some(Box::new(e))))?;
    std_sock
        .set_nonblocking(true)
        .map_err(|e| Error::new_bind(Some(Box::new(e))))?;

    let socket = UdpSocket::from_std(std_sock).map_err(|e| Error::new_bind(Some(Box::new(e))))?;

    Ok(socket)
}
