mod dht_builder;
pub use dht_builder::DhtBuilder;

use chrono::Utc;
use futures::{stream::FuturesUnordered, StreamExt};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::Write,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};
use tokio::{
    net::{lookup_host, UdpSocket}, sync::Semaphore, time::sleep
};
use yiilian_core::{
    common::{
        error::{Error, Kind},
        expect_log::ExpectLog,
        shutdown::ShutdownReceiver,
        util::random_bytes,
    },
    net::block_list::{BlockAddr, BlockList},
};

use crate::{
    common::{
        IPV4Consensus, Id, State,
        {
            dht_ctx_drop, dht_ctx_insert, dht_ctx_routing_tbl, dht_ctx_settings, dht_ctx_state,
            dht_ctx_trans_mgr, Context,
        },
        Settings,
    },
    data::body::{KrpcBody, Reply},
    net::{Client, Server},
    peer::PeerManager,
    routing_table::{Node, Persist, RoutingTable},
    service::KrpcService,
    transaction::{GetPeersResult, TransactionManager},
};

#[derive(Debug, Clone)]
pub enum DhtMode {
    Normal,
    Crawler(u16),
}

pub struct Dht<S> {
    ctx_index: u16,

    pub local_addr: SocketAddr,

    server: Server<S>,

    /// 保存 dht routing_table 已验证节点的文件
    nodes_file: PathBuf,
}

impl<S> Dht<S>
where
    S: KrpcService<KrpcBody, ResBody = KrpcBody, Error = Error> + Clone + Send + 'static,
{
    pub fn init(
        local_addr: SocketAddr,
        service: S,
        settings: Settings,
        node_block_list: Option<HashSet<BlockAddr>>,
        shutdown_rx: ShutdownReceiver,
        workers: Option<usize>,
        mode: DhtMode,
    ) -> Result<Self, Error> {
        let local_id = Id::from_ip(&local_addr.ip());
        let ctx_index = local_addr.port();

        let transaction_manager =
            TransactionManager::new(local_addr.port(), local_addr, mode);

        let routing_table = build_routing_table(
            ctx_index,
            local_id,
            settings.block_list_max_size,
            settings.bucket_size,
            node_block_list,
            shutdown_rx.clone(),
        );

        let state = build_state(local_id, settings.token_secret_size)?;

        let peer_manager = Mutex::new(PeerManager::new(
            settings.max_resources,
            settings.max_peers_per_resource,
        ));

        let socket = build_socket(local_addr)?;
        let socket = Arc::new(socket);

        let client = Client::new(socket.clone());

        let ctx = Context::new(
            settings,
            state,
            routing_table,
            peer_manager,
            transaction_manager,
            client,
        );
        dht_ctx_insert(ctx_index, ctx);

        let workers = match workers {
            Some(val) => {
                let workers = Arc::new(Semaphore::new(val));
                Some(workers)
            }
            None => None,
        };

        let server = Server::new(socket.clone(), service, workers);

        let nodes_file = home::home_dir()
            .map_or(
                Err(Error::new_path(
                    None,
                    Some("<user home> not found".to_owned()),
                )),
                |v| Ok(v),
            )?
            .join(format!(".yiilian/dht/{}.txt", ctx_index));

        Ok(Dht {
            ctx_index,
            local_addr,
            server,
            nodes_file,
        })
    }

    pub async fn run_loop(&self) {
        let ctx_index = self.ctx_index;
        // let shutdown_rx = self.shutdown_rx.clone();
        // let nodes_file = self.nodes_file.clone();

        // // graceful shutdown
        // tokio::spawn(async move {
        //     log::trace!(target: "yiilian_dht::dht::run_loop", "Task '{}' starting up", "persist nodes on exit");
        //     tokio::select! {
        //         _ = shutdown_rx.watch() => {
        //             persist_nodes(ctx_index, nodes_file.clone()).await;
        //             dht_ctx_drop(ctx_index);
        //         },
        //     }
        // });

        // 各种周期性的 future
        // tokio::try_join! 全部完成或有一个 Err 时退出
        match tokio::try_join!(
            self.server.run_loop(),
            self.ping_persist_nodes_once(),
            self.periodic_router_ping(),
            self.periodic_buddy_ping(),
            self.periodic_find_node(),
            self.periodic_ip4_maintenance(),
            self.periodic_token_rotation(),
            dht_ctx_trans_mgr(self.ctx_index).request_cleanup(),
        ) {
            Ok(_) => (),
            Err(e) => {
                log::debug!(target: "yiilian_dht::dht::run_loop", "[{}] Quit with error: {}", ctx_index, e);
            }
        }
    }

    /// Build and send a ping to a target. Doesn't wait for a response
    /// 生成并发任务执行 ping 请求，并等待响应
    async fn ping_persist_nodes_once(&self) -> Result<(), Error> {
        match fs::read_to_string(&self.nodes_file) {
            Ok(val) => {
                if let Ok(persist) = serde_yaml::from_str::<Persist>(&val) {
                    log::trace!(target: "yiilian_dht::dht::ping_persist_once", " [{}] Enter ping_persist_once", self.ctx_index);

                    for node_addr in persist.node_addrs {
                        let rst = dht_ctx_trans_mgr(self.ctx_index)
                            .ping_no_wait(node_addr, None)
                            .await;

                        match rst {
                            Err(e) => {
                                log::debug!(target: "yiilian_dht::dht::ping_persist_once", "[{}] ping_no_wait error: {}", self.ctx_index, e);
                            }
                            _ => (),
                        }
                        sleep(Duration::from_millis(10)).await;
                    }
                } else {
                    log::debug!(target: "yiilian_dht::dht::ping_persist_once", "[{}] Parsing node file {:?} error", self.ctx_index, self.nodes_file.as_os_str());
                }
            }
            Err(e) => {
                // 第一次运行的时候肯定是不存在 node_file 的
                log::debug!(target: "yiilian_dht::dht::ping_persist_once", "[{}] read node file error: {}", self.ctx_index, e);
            }
        }

        Ok(())
    }

    /// 周期性 ping 入口 router，可以获取对方识别的我们外网 ip，并且有机会让对方将我们加入 routing table
    /// 同时，在 ping 反馈时，我们也会将对方加入 routing table
    async fn periodic_router_ping(&self) -> Result<(), Error> {
        loop {
            let is_join_kad = dht_ctx_state(self.ctx_index)
                .read()
                .expect_error("dht_ctx_state.read() failed")
                .is_join_kad;

            let router_ping_interval_sec = {
                if is_join_kad {
                    dht_ctx_settings(self.ctx_index).router_ping_interval_secs
                } else {
                    dht_ctx_settings(self.ctx_index).router_ping_if_not_join_interval_secs
                }
            };

            log::trace!(
                target: "yiilian_dht::dht::periodic_router_ping",
                "[{}] Enter periodic_router_ping, is_join_kad: {}, interval_sec: {}",
                self.ctx_index, is_join_kad, router_ping_interval_sec
            );

            self.ping_routers().await;

            sleep(Duration::from_secs(router_ping_interval_sec)).await;
        }
    }

    /// 按照随即顺序 ping 多个 router，直到所有的 ping 请求对方都响应了消息，或者 ping 请求在接收响应前超时
    /// FuturesUnordered类型允许 Future 以任意顺序执行
    async fn ping_routers(&self) {
        let mut futures = FuturesUnordered::new();
        // 入口 router
        let routers = &dht_ctx_settings(self.ctx_index).routers;

        for hostname in routers {
            futures.push(self.ping_router(hostname.clone()));
        }

        while let Some(rst) = futures.next().await {
            match rst {
                Err(e) => {
                    log::debug!(target:"yiilian_dht::dht::ping_routers", "[{}] error: {:?}", self.ctx_index, e);
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
                        dht_ctx_routing_tbl(self.ctx_index)
                            .lock()
                            .expect_error("dht_ctx_routing_tbl.lock() failed")
                            .white_list
                            .insert(socket_addr.ip());

                        // 生成并发任务执行 ping 请求，并等待响应
                        dht_ctx_trans_mgr(self.ctx_index)
                            .ping_no_wait(socket_addr, None)
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
        // 每隔 10 秒做一次 ping 检查
        let ping_check_interval_secs = dht_ctx_settings(self.ctx_index).ping_check_interval_secs;

        loop {
            sleep(Duration::from_secs(ping_check_interval_secs)).await; // 由于有这个 sleep，在它挂起任务时，就有机会优雅退出

            let is_join_kad = dht_ctx_state(self.ctx_index)
                .read()
                .expect_error("dht_ctx_state.read() failed")
                .is_join_kad;

            if !is_join_kad {
                continue;
            }

            log::trace!(target: "yiilian_dht::dht::periodic_buddy_ping", "[{}] Enter periodic_buddy_ping", self.ctx_index);

            // 将需要状态的东西打包到一个块中，这样 Rust 就不会抱怨 MutexGuard 跨 .await 了
            let reverify_interval_secs = {
                let reverify_grace_period_secs =
                    dht_ctx_settings(self.ctx_index).reverify_grace_period_secs;
                let verify_grace_period_secs =
                    dht_ctx_settings(self.ctx_index).verify_grace_period_secs;

                // 将过期没再次校验的节点从 buckets 中删除
                dht_ctx_routing_tbl(self.ctx_index)
                    .lock()
                    .expect_error("dht_ctx_routing_tbl.lock() failed")
                    .prune(
                        Duration::from_secs(reverify_grace_period_secs), // 每隔 14 分钟一次
                        Duration::from_secs(verify_grace_period_secs),   // 每隔 1 分钟一次
                    );

                // 验证的有效时间为 15 分钟
                dht_ctx_settings(self.ctx_index).reverify_interval_secs
            };

            // 到了 reverify_interval_secs 再次验证时间间隔，需要将所有的 node （已验证/未验证） 都 ping 一遍
            // 超过 ping_if_older_than 时间点的节点，都需要被 ping
            let ping_if_older_than = Utc::now() - Duration::from_secs(reverify_interval_secs);

            let (unverified, verified) = {
                let unverified = dht_ctx_routing_tbl(self.ctx_index)
                    .lock()
                    .expect_error("dht_ctx_routing_tbl.lock() failed")
                    .get_all_unverified();
                let verified = dht_ctx_routing_tbl(self.ctx_index)
                    .lock()
                    .expect_error("dht_ctx_routing_tbl.lock() failed")
                    .get_all_verified();

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
                let rst = dht_ctx_trans_mgr(self.ctx_index)
                    .ping_no_wait(node.address, Some(node.id))
                    .await;

                match rst {
                    Err(error) => match error.get_kind() {
                        Kind::Transatcion => (),
                        _ => {
                            log::debug!(target:"yiilian_dht::dht::periodic_buddy_ping", "[{}] Error ping unverified: {:?}", self.ctx_index, error);
                        }
                    },
                    Ok(_) => {}
                }
            }

            // Reverify those who haven't been verified recently
            for node in verified {
                if let Some(last_verified) = node.last_verified {
                    if last_verified >= ping_if_older_than {
                        continue;
                    }
                }

                let rst = dht_ctx_trans_mgr(self.ctx_index)
                    .ping_no_wait(node.address, Some(node.id))
                    .await;

                match rst {
                    Err(error) => match error.get_kind() {
                        Kind::Transatcion => {}
                        _ => {
                            log::debug!(target:"yiilian_dht::dht::periodic_buddy_ping", "[{}] Error ping verified: {:?}", self.ctx_index, error);
                        }
                    },
                    _ => (),
                }
            }
        }
    }

    /// 周期性 find_node 一个随机生成的接近本机节点的 Node id
    async fn periodic_find_node(&self) -> Result<(), Error> {
        let find_node_interval_secs = dht_ctx_settings(self.ctx_index).find_nodes_interval_secs; // 33 s
        loop {
            sleep(Duration::from_secs(find_node_interval_secs)).await;
            let is_join_kad = dht_ctx_state(self.ctx_index)
                .read()
                .expect_error("dht_ctx_state.read() failed")
                .is_join_kad;
            if !is_join_kad {
                continue;
            }

            log::trace!(target: "yiilian_dht::dht::periodic_find_node", "[{}] Enter periodic_find_node", self.ctx_index);

            let (count_unverified, count_verified) = dht_ctx_routing_tbl(self.ctx_index)
                .lock()
                .expect_error("dht_ctx_routing_tbl.lock() failed")
                .count();

            // 如果路由表中没有 node ，则 ping 入口 router。
            // 当我们已经睡了一段时间并且失去了所有节点，这会很有帮助。
            if count_verified == 0 {
                self.ping_routers().await;
            }

            // 有足够多的未验证节点，则不需要本次 find_node 了
            let id_near_us = {
                let find_nodes_skip_count = dht_ctx_settings(self.ctx_index).find_nodes_skip_count;
                if count_unverified > find_nodes_skip_count {
                    continue;
                }

                // 生成一个和本机 ID 接近的新 ID （只有后 4 个字节不同）
                let id_near = dht_ctx_state(self.ctx_index)
                    .read()
                    .expect_error("dht_ctx_state.read() failed")
                    .get_local_id()
                    .make_mutant(4);
                let id_near = id_near.expect_error("id_near make_mutant() error");

                id_near
            };

            // 向这些附近节点中发送 find_node 本机节点的请求
            dht_ctx_trans_mgr(self.ctx_index)
                .find_node(id_near_us)
                .await;
        }
    }

    /// 每隔 10 秒，周期性维护 IPv4 （使用本机的最佳外网IP地址生成本机节点 ID）
    async fn periodic_ip4_maintenance(&self) -> Result<(), Error> {
        let ip4_maintenance_interval_sec =
            dht_ctx_settings(self.ctx_index).ip4_maintenance_interval_sec;

        loop {
            sleep(Duration::from_secs(ip4_maintenance_interval_sec)).await;
            log::trace!(target: "yiilian_dht::dht::periodic_ip4_maintenance", "[{}] Enter periodic_ip4_maintenance", self.ctx_index);

            // 每隔 10 秒，将各 ip 投票数 - 1
            dht_ctx_state(self.ctx_index)
                .write()
                .expect_error("dht_ctx_state.write() failed")
                .ip4_source
                .decay();

            let best_ipv4 = dht_ctx_state(self.ctx_index)
                .read()
                .expect_error("dht_ctx_state.read() failed")
                .ip4_source
                .get_best_ipv4();
            if let Some(ip) = best_ipv4 {
                // 取出被投票数最多的外网 ipv4 地址，如果获取的投票数没超过阈值，则返回 None
                let ip = IpAddr::V4(ip);
                let local_id = dht_ctx_state(self.ctx_index)
                    .read()
                    .expect_error("dht_ctx_state.read() failed")
                    .get_local_id();

                // 如果本机外网 ip 地址和 本机节点 id 没有有效匹配，则生成一个新的有效匹配的本机 node id
                let is_not_valid = !local_id.is_valid_for_ip(
                    &ip,
                    &dht_ctx_routing_tbl(self.ctx_index)
                        .lock()
                        .expect_error("dht_ctx_routing_tbl.lock() failed")
                        .white_list,
                );

                if is_not_valid {
                    let new_id = Id::from_ip(&ip);
                    dht_ctx_state(self.ctx_index)
                        .write()
                        .expect_error("dht_ctx_state.write() failed")
                        .set_local_id(new_id);

                    dht_ctx_routing_tbl(self.ctx_index)
                        .lock()
                        .expect_error("dht_ctx_routing_tbl.lock() failed")
                        .set_id(new_id);
                }
            }
        }
    }

    /// 定期维护 token
    async fn periodic_token_rotation(&self) -> Result<(), Error> {
        let token_refresh_interval_sec =
            dht_ctx_settings(self.ctx_index).token_refresh_interval_sec;

        loop {
            log::trace!(target: "yiilian_dht::dht::periodic_token_rotation", "[{}] Enter periodic_token_rotation", self.ctx_index);

            sleep(Duration::from_secs(token_refresh_interval_sec)).await;
            self.rotate_token_secrets();
        }
    }

    /// 更新 token_secret
    fn rotate_token_secrets(&self) {
        let new_token_secret = random_bytes(dht_ctx_settings(self.ctx_index).token_secret_size);
        let old_token_secret = dht_ctx_state(self.ctx_index)
            .read()
            .expect_error("dht_ctx_state.read() failed")
            .token_secret
            .clone();

        dht_ctx_state(self.ctx_index)
            .write()
            .expect_error("dht_ctx_state.read() failed")
            .old_token_secret = old_token_secret;

        dht_ctx_state(self.ctx_index)
            .write()
            .expect_error("dht_ctx_state.read() failed")
            .token_secret = new_token_secret;
    }

    pub async fn get_peers(&self, info_hash: Id) -> Result<GetPeersResult, Error> {
        dht_ctx_trans_mgr(self.ctx_index)
            .get_peers(info_hash, true)
            .await
    }
}

impl<S> Drop for Dht<S> {
    fn drop(&mut self) {
        let ctx_index = self.ctx_index;
        let nodes_file = self.nodes_file.clone();

        // save nodes
        log::trace!(target: "yiilian_dht::dht::run_loop", "Task '{}' starting up", "persist nodes on exit");
        persist_nodes(ctx_index, nodes_file.clone());
        dht_ctx_drop(ctx_index);
    }
}

fn build_routing_table(
    ctx_index: u16,
    local_id: Id,
    block_list_max_size: usize,
    bucket_size: usize,
    node_block_list: Option<HashSet<BlockAddr>>,
    shutdown_rx: ShutdownReceiver,
) -> Mutex<RoutingTable> {
    let node_block_list = BlockList::new("node_block_list", block_list_max_size, node_block_list, shutdown_rx);
    let routing_table = RoutingTable::new(ctx_index, bucket_size, node_block_list, local_id);

    Mutex::new(routing_table)
}

fn build_state(local_id: Id, token_secret_size: usize) -> Result<RwLock<State>, Error> {
    let token_secret = random_bytes(token_secret_size);

    Ok(RwLock::new(State::new(
        local_id,
        IPV4Consensus::new(2, 10),
        token_secret,
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

/// save nodes to file
fn persist_nodes(ctx_index: u16, nodes_file: PathBuf) {
    let mut nodes = dht_ctx_routing_tbl(ctx_index)
        .lock()
        .expect_error("dht_ctx_routing_tbl.lock() failed")
        .get_all_verified();
    nodes.extend(
        dht_ctx_routing_tbl(ctx_index)
            .lock()
            .expect_error("dht_ctx_routing_tbl.lock() failed")
            .get_all_unverified(),
    );

    let node_addrs: Vec<SocketAddr> = nodes.into_iter().map(|node| node.address).collect();

    let persist = Persist { node_addrs };

    let persist = serde_yaml::to_string(&persist).expect_error("serde_yaml::to_string() failed");
    let parent_path = nodes_file
        .parent()
        .expect_error("nodes_file.parent() is none");

    match std::fs::create_dir_all(&parent_path) {
        Ok(_) => {
            let mut f = File::create(&nodes_file).expect_error("File::create() node file failed");

            f.write_all(persist.as_bytes())
                .expect_error("f.write_all() nodes failed");
        }
        Err(e) => {
            log::error!(target:"yiilian_dht::routing_table::save_nodes", "Path create {:?} error: {}", parent_path, e);
        }
    }
}

pub async fn ping(
    ctx_index: u16,
    target_addr: SocketAddr,
    target_id: Option<Id>,
) -> Result<Reply, Error> {
    dht_ctx_trans_mgr(ctx_index)
        .ping(target_addr, target_id)
        .await
}

pub async fn find_node(ctx_index: u16, target_id: Id) -> Result<Vec<Node>, Error> {
    let rst = dht_ctx_trans_mgr(ctx_index).find_node(target_id).await;
    Ok(rst)
}

pub async fn get_peers(
    ctx_index: u16,
    info_hash: Id,
    quick_mode: bool,
) -> Result<GetPeersResult, Error> {
    dht_ctx_trans_mgr(ctx_index)
        .get_peers(info_hash, quick_mode)
        .await
}

pub async fn announce_peer(
    ctx_index: u16,
    local_addr: SocketAddr,
    info_hash: Id,
) -> Result<Vec<Node>, Error> {
    dht_ctx_trans_mgr(ctx_index)
        .announce_peer(info_hash, Some(local_addr.port()))
        .await
}
