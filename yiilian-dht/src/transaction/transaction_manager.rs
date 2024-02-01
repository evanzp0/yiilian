use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, SocketAddr},
    sync::Mutex,
    time::Duration,
};

use chrono::Utc;
use tokio::{sync::oneshot, time::interval};
use yiilian_core::{
    common::{error::Error, shutdown::ShutdownReceiver}, data::Request, except_option, except_result
};

use crate::{
    common::{
        {
            dht_ctx_client, dht_ctx_peer_mgr, dht_ctx_routing_tbl, dht_ctx_settings, dht_ctx_state,
            dht_ctx_trans_mgr,
        },
        Id,
        calculate_token,
    }, data::{
        announce_peer::AnnouncePeer,
        body::{BodyKind, KrpcBody, Query, Reply},
        find_node::FindNode,
        find_node_reply::FindNodeReply,
        get_peers::GetPeers,
        get_peers_reply::GetPeersReply,
        ping::Ping,
        ping_announce_replay::PingOrAnnounceReply,
        util::reply_matches_query,
    }, routing_table::{Buckets, Node}
};

use super::{GetPeersResponder, GetPeersResult, Transaction, TransactionId};

#[derive(Debug)]
/// 管理所有的事务性和非事务性的发送和接受的消息
pub struct TransactionManager {
    ctx_index: u16,
    local_addr: SocketAddr,
    /// 对外发送 query 的事务队列（只有主动发送 query 时才会产生事务）
    transactions: Mutex<HashMap<TransactionId, Transaction>>,
}

impl TransactionManager {
    pub fn new(ctx_index: u16, local_addr: SocketAddr, _shutdown: ShutdownReceiver) -> Self {
        let transactions = Mutex::new(HashMap::new());

        Self {
            ctx_index,
            local_addr,
            transactions,
        }
    }

    /// 清除早于 duration 的请求事务
    pub fn prune_older_than(&self, duration: Duration) {
        // 过期时间点 = 当前时间 - duration
        let time = Utc::now() - duration;

        let _len_before = except_result!(self.transactions.lock(), "transactions.lock() error").len();

        // 保留创建时间晚于 time 的事务
        except_result!(self.transactions.lock(), "transactions.lock() error")
            .retain(|_, v| -> bool { v.created_at >= time }); 
        let _len_after = except_result!(self.transactions.lock(), "transactions.lock() error").len();
    }

    /// 检查 query 查询是否已经在事务处理中了
    fn check_query_in_trans(&self, dest: &SocketAddr, query: &Query) -> bool {
        except_result!(self.transactions.lock(), "transactions.lock() error").iter().any(|t| {
            let tran = t.1;
            if tran.addr == *dest && tran.message == *query {
                true
            } else {
                false
            }
        })
    }

    pub(crate) async fn send_query(
        &self,
        query: Query,
        dest: &SocketAddr,
        dest_id: Option<Id>,
        timeout: Option<Duration>,
    ) -> Result<Reply, Error> {
        if self.check_query_in_trans(dest, &query) {
            Err(Error::new_transaction(&format!(
                "Transaction of query exists: {:?}",
                dest
            )))?
        }

        // 添加事务
        let (notify_tx, notify_rx) = oneshot::channel::<Reply>();
        let transaction = Transaction::new(
            query.get_tid(),
            dest_id.clone(),
            dest.to_owned(),
            query.clone(),
            Some(notify_tx),
        );
        let tran_id = transaction.id.clone();
        self.add_transaction(transaction);

        let rst = match timeout {
            Some(timeout) => {
                match tokio::time::timeout(
                    timeout,
                    self.send_query_internal(&query, dest, notify_rx),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_) => Err(Error::new_timeout(&format!(
                        "Timed out after {:?} waiting for {} to respond to {:?}",
                        timeout, dest, query
                    ))),
                }
            }
            None => self.send_query_internal(&query, dest, notify_rx).await,
        };

        match rst {
            Ok(_) => rst,
            Err(e) => {
                // 发生错误时删除对应事务(正常返回的 reply 的事务，在 handle_reply 中已经被删除了)
                self.remove_transcation(&tran_id);
                // 并将目标节点加入 block_list，同时从 routing_table 中删除
                let timeout_block_duration_sec =
                    dht_ctx_settings(self.ctx_index).timeout_block_duration_sec;
                except_result!(
                    dht_ctx_routing_tbl(self.ctx_index).lock(),
                    "Lock context routing_table failed"
                )
                .add_block_list(
                    *dest,
                    dest_id,
                    Some(Duration::from_secs(timeout_block_duration_sec)),
                );

                Err(e)
            }
        }
    }

    /// 无需等待回复发送 query 请求
    pub(crate) async fn send_query_no_wait(
        &self,
        query: Query,
        dest: &SocketAddr,
        dest_id: Option<Id>,
    ) -> Result<usize, Error> {
        if self.check_query_in_trans(dest, &query) {
            Err(Error::new_transaction(&format!(
                "Transaction of query exists: {:?}",
                dest
            )))?
        }

        // 添加事务
        let transaction = Transaction::new(
            query.get_tid(),
            dest_id.clone(),
            dest.to_owned(),
            query.clone(),
            None,
        );
        let tran_id = transaction.id.clone();
        self.add_transaction(transaction);

        let rst = self.send_query_internal_no_wait(&query, dest).await;

        match rst {
            Ok(_) => rst,
            Err(e) => {
                // 超时时删除对应事务(正常返回的 reply 的事务，在 handle_reply 中已经被删除了)
                self.remove_transcation(&tran_id);

                Err(e)
            }
        }
    }

    fn remove_transcation(&self, tran_id: &TransactionId) -> Option<Transaction> {
        let tran = except_result!(self.transactions.lock(), "transactions.lock() error").remove(&tran_id);
        match tran {
            Some(tran) => Some(tran),
            None => None,
        }
    }

    async fn send_query_internal(
        &self,
        query: &Query,
        dest: &SocketAddr,
        notify_rx: oneshot::Receiver<Reply>,
    ) -> Result<Reply, Error> {
        let req = Request::new(
            KrpcBody::new(BodyKind::Query(query.to_owned())),
            dest.to_owned(),
            self.local_addr,
        );

        dht_ctx_client(self.ctx_index).send(req).await?;

        // 等待 transaction 上的 reply
        match notify_rx.await {
            Ok(reply) => Ok(reply),
            Err(_) => Err(Error::new_transaction(&format!(
                "Transaction is closed: {:?}",
                dest
            ))),
        }
    }

    async fn send_query_internal_no_wait(
        &self,
        query: &Query,
        dest: &SocketAddr,
    ) -> Result<usize, Error> {
        let req = Request::new(
            KrpcBody::new(BodyKind::Query(query.to_owned())),
            dest.to_owned(),
            self.local_addr,
        );

        dht_ctx_client(self.ctx_index).send(req).await
    }

    /// Adds a 'vote' for whatever IP address the sender says we have.
    /// addr：对方 IP
    /// requester_ip 是对方看到的本机的外网 IP
    fn ip4_vote_helper(ctx_index: u16, addr: &SocketAddr, requester_ip: &Option<SocketAddr>) {
        if let IpAddr::V4(their_ip) = addr.ip() {
            if let Some(SocketAddr::V4(they_claim_our_sockaddr)) = &requester_ip {
                except_result!(
                    dht_ctx_state(ctx_index).write(),
                    "dht_ctx_state().write() failed"
                )
                .ip4_source
                .add_vote(their_ip, *they_claim_our_sockaddr.ip());
            }
        }
    }

    /// 添加事务
    pub(crate) fn add_transaction(&self, tran: Transaction) {
        except_result!(self.transactions.lock(), "transactions.lock() error")
            .insert(tran.get_id().clone(), tran);
    }

    /// 处理对方的 ping 请求
    pub async fn handle_ping(
        &self,
        query: &Ping,
        remote_addr: &SocketAddr,
    ) -> Result<(Reply, SocketAddr), Error> {
        // info!("Receive ping request from {:?}", sender);

        let local_id = except_result!(
            dht_ctx_state(self.ctx_index).read(),
            "dht_ctx_state.read() failed"
        )
        .get_local_id();
        let read_only = dht_ctx_settings(self.ctx_index).read_only;

        let reply = PingOrAnnounceReply {
            t: query.t.clone(),
            v: None,
            ip: Some(remote_addr.to_owned()),
            ro: if read_only { Some(1) } else { None },
            id: local_id,
        };

        Ok((Reply::PingOrAnnounce(reply), remote_addr.clone()))
    }

    /// 处理对方的 find_node 请求
    pub async fn handle_find_node(
        &self,
        query: &FindNode,
        remote_addr: &SocketAddr,
    ) -> Result<(Reply, SocketAddr), Error> {
        let local_id = except_result!(
            dht_ctx_state(self.ctx_index).read(),
            "dht_ctx_state.read() failed"
        )
        .get_local_id();
        let read_only = dht_ctx_settings(self.ctx_index).read_only;

        //获取除 requester_id 外，距离 target 最近的节点
        let nearest = except_result!(
            dht_ctx_routing_tbl(self.ctx_index).lock(),
            "dht_ctx_routing_tbl.lock() failed"
        )
        .get_nearest_nodes(&query.target, Some(&query.id));

        let reply = FindNodeReply {
            t: query.t.clone(),
            v: None,
            ip: Some(remote_addr.to_owned()),
            ro: if read_only { Some(1) } else { None },
            id: local_id,
            nodes: nearest,
        };

        Ok((Reply::FindNode(reply), remote_addr.clone()))
    }

    /// 处理对方 get_peers 请求
    pub(crate) async fn handle_get_peers(
        &self,
        query: &GetPeers,
        remote_addr: &SocketAddr,
    ) -> Result<(Reply, SocketAddr), Error> {
        let local_id = except_result!(
            dht_ctx_state(self.ctx_index).read(),
            "dht_ctx_state.read() failed"
        )
        .get_local_id();
        let read_only = dht_ctx_settings(self.ctx_index).read_only;

        let peers = {
            let get_peers_freshness_secs =
                dht_ctx_settings(self.ctx_index).get_peers_freshness_secs;
            let max_peers_response = dht_ctx_settings(self.ctx_index).max_peers_response;
            let newer_than = Utc::now() - Duration::from_secs(get_peers_freshness_secs);
            let mut peers = except_result!(
                dht_ctx_peer_mgr(self.ctx_index).lock(),
                "dht_ctx_peer_mgr.lock() failed"
            )
            .get_peers(&query.info_hash, Some(newer_than));
            peers.truncate(max_peers_response);
            peers
        };

        // 根据 token_secret 和对方 IP 生成 token，对方在向我方发出 announce 请求中需要带上该 token
        let token_secret = except_result!(
            dht_ctx_state(self.ctx_index).read(),
            "dht_ctx_state.read() failed"
        )
        .token_secret
        .clone();
        let token = calculate_token(&remote_addr, token_secret);
        let token = token.to_vec().into();
        let nearest_nodes = except_result!(
            dht_ctx_routing_tbl(self.ctx_index).lock(),
            "dht_ctx_routing_tbl.lock() failed"
        )
        .get_nearest_nodes(&query.info_hash, Some(&query.id));
        let reply = GetPeersReply {
            t: query.t.clone(),
            v: None,
            ip: Some(remote_addr.to_owned()),
            ro: if read_only { Some(1) } else { None },
            id: local_id,
            token: token,
            nodes: nearest_nodes,
            values: peers,
        };

        Ok((Reply::GetPeers(reply), remote_addr.clone()))
    }

    pub(crate) async fn handle_announce_peer(
        &self,
        query: &AnnouncePeer,
        remote_addr: &SocketAddr,
    ) -> Result<(Reply, SocketAddr), Error> {
        let local_id = except_result!(
            dht_ctx_state(self.ctx_index).read(),
            "dht_ctx_state.read() failed"
        )
        .get_local_id();
        let read_only = dht_ctx_settings(self.ctx_index).read_only;

        // 根据 token_secret/old_token_secret 和 对方 ip 计算 token 是否合法（这样就不用缓存上次发出的 token）
        let token_secret = except_result!(
            dht_ctx_state(self.ctx_index).read(),
            "dht_ctx_state.read() failed"
        )
        .token_secret
        .clone();
        let old_token_secret = except_result!(
            dht_ctx_state(self.ctx_index).read(),
            "dht_ctx_state.read() failed"
        )
        .old_token_secret
        .clone();
        let is_token_valid = query.token == calculate_token(&remote_addr, token_secret).to_vec()
            || query.token == calculate_token(&remote_addr, old_token_secret).to_vec();

        if is_token_valid {
            // 如果有 implied_port，则使用请求方的 ip+port; 如果没有 implied_port，则使用请求消息中的 port
            let sockaddr = match query.implied_port {
                Some(implied_port) if implied_port == 1 => remote_addr.clone(),
                _ => {
                    let mut tmp = remote_addr.clone();
                    tmp.set_port(query.port);
                    tmp
                }
            };

            // 将对方 address 加入到 announce 的 info_hash 对应的 peers 列表中
            except_result!(
                dht_ctx_peer_mgr(self.ctx_index).lock(),
                "dht_ctx_peer_mgr.lock() failed"
            )
            .announce_peer(query.info_hash, sockaddr);

            let reply = PingOrAnnounceReply {
                t: query.t.clone(),
                v: None,
                ip: Some(remote_addr.clone()),
                ro: if read_only { Some(1) } else { None },
                id: local_id,
            };

            Ok((Reply::PingOrAnnounce(reply), remote_addr.clone()))
        } else {
            Err(Error::new_token(&format!(
                "Invalid token: {:?}",
                query.token
            )))
        }
    }

    /// 处理对方的反馈（需要事务处理）
    pub(crate) async fn handle_reply(
        &self,
        reply: &Reply,
        sender: &SocketAddr,
    ) -> Result<(), Error> {
        // 从响应中获取对方节点 ID
        let their_id = reply.get_id();

        // 仅当节点的 id 对于其 IP 有效时，它才适合加入 kbucket 并在 IPv4 上进行投票。
        let id_is_valid = {
            their_id.is_valid_for_ip(
                &sender.ip(),
                &except_result!(
                    dht_ctx_routing_tbl(self.ctx_index).lock(),
                    "dht_ctx_routing_tbl.lock() failed"
                )
                .white_list,
            )
        };
        // log::trace!(target: "yiilian_dht::handle_reply", "sender: {},  it's id {} is valid : {}", sender, their_id, id_is_valid);

        if id_is_valid && sender.port() > 0 {
            // 根据这次的reply，对我们的外网IP增加 vote，注意 reply.requester_ip 是对方认为我们的外网 IP
            Self::ip4_vote_helper(self.ctx_index, &sender, &reply.get_ip());

            // 将对方节点及ipport，加入或更新 kbucket
            // 由于对方节点时响应我们的请求的，所以它就是 verified node, 因此 add_or_update(_, verified) 参数要传 true
            except_result!(
                dht_ctx_routing_tbl(self.ctx_index).lock(),
                "dht_ctx_routing_tbl.lock() failed"
            )
            .add_or_update(Node::new(their_id, *sender), true)?;
        }

        // 如果不在黑名单中，且事务有回传 channel , 则通过该 channel 回传 reply
        // take_matching_transaction 会将匹配 reply 的 transaction 删除
        if let Some(transaction) = self.take_matching_transaction(&reply, sender) {
            let in_block_list = except_result!(
                dht_ctx_routing_tbl(self.ctx_index).lock(),
                "dht_ctx_routing_tbl.lock() failed"
            )
            .is_blocked(sender);

            if !in_block_list {
                if let Some(response_channel) = transaction.response_channel {
                    response_channel.send(reply.clone()).map_err(|_| {
                        Error::new_transaction(&format!(
                            "Transaction({:?}) is closed",
                            hex::encode(transaction.id.get_bytes())
                        ))
                    })?;
                }
            }
        }

        Ok(())
    }

    /// 根据事务 ID 匹配发送时和接收时的对方 ID 是否一致
    pub(crate) fn take_matching_transaction(
        &self,
        reply: &Reply,
        src_addr: &SocketAddr,
    ) -> Option<Transaction> {
        if self.matching_transaction(reply, src_addr) {
            let tid = reply.get_tid();
            let transaction = { self.remove_transcation(&tid) };
            return transaction;
        }

        None
    }

    /// 根据消息中的事务 ID 获取发送和接收时 IP 匹配的 Transaction
    pub(crate) fn matching_transaction(&self, reply: &Reply, src_addr: &SocketAddr) -> bool {
        let tid = reply.get_tid();
        let transactions = except_result!(self.transactions.lock(), "transactions.lock() error");
        let transaction = transactions.get(&tid);

        // Is there a matching transaction id in storage?
        if let Some(transaction) = transaction {
            // Did this response come from the expected IP address?
            if transaction.addr == *src_addr {
                let sender_id = reply.get_id();

                // Does the Id of the sender match the recorded addressee of the original request (if any)?
                // 当发送时对方的 node id 为空，或对方的 node id 为空且对方 node id 和 reply 中的发送方 node id 相同时
                if transaction.node_id.is_none() // 当 ping route 时，node_id 为空
                    || (!transaction.node_id.is_none() && except_option!(transaction.node_id.as_ref(), "node_id.as_ref() error") == &sender_id)
                {
                    // Does the reply type match the query type?
                    if reply_matches_query(&transaction.message, reply) {
                        return true;
                    } else {
                        log::trace!(target: "yiilian_dht::transaction", "reply not match query, tid: {:?}, address: {}", transaction.id.0, src_addr);
                    }
                }
            }
        }

        false
    }

    /// 发出 ping query
    pub(crate) async fn ping(
        &self,
        target_addr: SocketAddr,
        target_id: Option<Id>,
    ) -> Result<Reply, Error> {
        // log::trace!(target:"yiilian_dht::operation", "ping to {} with target_id : {:?}", target_addr, target_id);

        if target_addr.port() == 0 {
            Err(Error::new_general(&format!("ping target_addr is invalid: {}", target_addr)))?
        }

        let local_id = except_result!(
            dht_ctx_state(self.ctx_index).read(),
            "dht_ctx_state.read() failed"
        )
        .get_local_id();
        let read_only = dht_ctx_settings(self.ctx_index).read_only;

        let ping_query = Ping {
            t: TransactionId::from_random(),
            v: None,
            ip: None,
            ro: if read_only { Some(1) } else { None },
            id: local_id,
        };

        let send_query_timeout_sec = dht_ctx_settings(self.ctx_index).send_query_timeout_sec;

        self.send_query(
            Query::Ping(ping_query),
            &target_addr,
            target_id.clone(),
            Some(Duration::from_secs(send_query_timeout_sec)), // 15 秒超时
        )
        .await
    }

    /// 发出 ping query
    pub(crate) async fn ping_no_wait(
        &self,
        target_addr: SocketAddr,
        target_id: Option<Id>,
    ) -> Result<(), Error> {
        // log::trace!(target:"yiilian_dht::operation", "ping to {} with target_id : {:?}", target_addr, target_id);

        if target_addr.port() == 0 {
            Err(Error::new_general(&format!("ping_no_wait target_addr is invalid: {}", target_addr)))?
        }

        let local_id = except_result!(
            dht_ctx_state(self.ctx_index).read(),
            "dht_ctx_state.read() failed"
        )
        .get_local_id();

        let read_only = dht_ctx_settings(self.ctx_index).read_only;
        let ping_query = Ping {
            t: TransactionId::from_random(),
            v: None,
            ip: None,
            ro: if read_only { Some(1) } else { None },
            id: local_id,
        };

        let rst = self
            .send_query_no_wait(Query::Ping(ping_query), &target_addr, target_id.clone())
            .await
            .map(|_| ());
        rst
    }

    /// 找到离目标节点最近的节点集合
    ///
    /// 这个操作会一直迭代，直到没有更近的节点被发现或者超时后才结束
    pub(crate) async fn find_node(&self, target_id: Id) -> Vec<Node> {
        // buckets 中存放的是 routing_table 中已验证的节点，以及本次 find_node 以来对方反馈的 nodes 节点
        let local_id = except_result!(
            dht_ctx_state(self.ctx_index).read(),
            "dht_ctx_state.read() failed"
        )
        .get_local_id();
        let bucket_size = dht_ctx_settings(self.ctx_index).bucket_size;

        let mut buckets = Buckets::new(bucket_size, local_id);

        let mut best_ids = Vec::<Id>::new();

        loop {
            // 每次循环都从 routing_table 中加载新的已验证的 node 到当前 buckets 中
            let all_verifyied = except_result!(
                dht_ctx_routing_tbl(self.ctx_index).lock(),
                "dht_ctx_routing_tbl.lock() failed"
            )
            .get_all_verified();
            for item in all_verifyied {
                if !buckets.contains(&item.id)
                    && !except_result!(
                        dht_ctx_routing_tbl(self.ctx_index).lock(),
                        "dht_ctx_routing_tbl.lock() failed"
                    )
                    .block_list
                    .contains(item.address.ip(), item.address.port())
                {
                    buckets.add(item, None).ok();
                }
            }

            // 在 buckets 中找到离 target_id 最近的节点，如果没找到任何节点，则稍后再尝试
            let nearest = buckets.get_nearest_nodes(&target_id, None);
            if nearest.is_empty() {
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }

            let best_ids_current: Vec<Id> = nearest.iter().map(|node| node.id.clone()).collect();

            if best_ids == best_ids_current {
                // 直到找不到更近的节点，则退出循环
                break;
            }
            best_ids = best_ids_current;

            // 发送 find_node 请求给这些节点
            let sender_id = local_id.clone().clone();
            let read_only = dht_ctx_settings(self.ctx_index).read_only;

            let todos = futures::stream::FuturesUnordered::new();
            for node in nearest {
                let read_only = if read_only { Some(1) } else { None };

                let find_node_query = (
                    Query::FindNode(FindNode {
                        t: TransactionId::from_random(),
                        v: None,
                        ip: None,
                        ro: read_only,
                        id: sender_id.clone(),
                        target: target_id.clone(),
                    }),
                    node.address,
                    node.id.clone(),
                );

                todos.push(find_node_query);
            }

            // 随机执行 todos 中的 future
            for (query, dest_addr, dest_id) in todos {
                let request_result = self
                    .send_query(
                        query,
                        &dest_addr,
                        Some(dest_id.clone()),
                        Some(Duration::from_secs(5)),
                    )
                    .await;

                match request_result {
                    Ok(reply) => match reply {
                        Reply::FindNode(val) => {
                            log::trace!(target: "yiilian_dht::transaction::find_node", "[{}] Address {:?} got {} nodes", self.local_addr.port(), dest_addr, val.nodes.len());

                            for node in val.nodes {
                                // 将 find_node 返回的 nodes 加入到 routing_table 中
                                let id_is_valid = {
                                    node.id.is_valid_for_ip(
                                        &node.address.ip(),
                                        &except_result!(
                                            dht_ctx_routing_tbl(self.ctx_index).lock(),
                                            "dht_ctx_routing_tbl.lock() failed"
                                        )
                                        .white_list,
                                    )
                                };

                                if id_is_valid && node.address.port() > 0 {
                                    if let Err(e) = except_result!(
                                        dht_ctx_routing_tbl(self.ctx_index).lock(),
                                        "dht_ctx_routing_tbl.lock() failed"
                                    )
                                    .add_or_update(node.clone(), false)
                                    {
                                        log::trace!(
                                            target: "yiilian_dht::transaction::find_node", 
                                            "[{}] Add node {:?} to buckets failed, error: {}", 
                                            self.local_addr.port(), node, e
                                        );
                                    }
                                }
                                if !buckets.contains(&node.id) {
                                    log::trace!(target: "yiilian_dht::transaction::find_node", 
                                        "[{}] Node (id: {:?}, {:?}) is a candidate for buckets", 
                                        self.local_addr.port(), node.id, node.address
                                    );

                                    buckets.add(node.clone(), None).ok();
                                }
                            }
                        }
                        _ => {
                            log::trace!(target: "yiilian_dht::transaction::find_node", "[{}] Address {:?} got wrong frame type back: {:?}", self.local_addr.port(), dest_addr, reply);

                            let reply_error_block_duration_sec =
                                dht_ctx_settings(self.ctx_index).reply_error_block_duration_sec;
                            buckets.remove(&dest_id);
                            except_result!(
                                dht_ctx_routing_tbl(self.ctx_index).lock(),
                                "dht_ctx_routing_tbl.lock() failed"
                            )
                            .add_block_list(
                                dest_addr,
                                Some(dest_id),
                                Some(Duration::from_secs(reply_error_block_duration_sec)),
                            );
                        }
                    },
                    Err(error) => {
                        // 已在 send_query() 中加入了黑名单
                        buckets.remove(&dest_id);

                        log::trace!(
                            target: "yiilian_dht::transaction::find_node",
                            "[{}] {:?} find_node error: {}",
                            self.local_addr.port(), dest_addr, error
                        );
                    }
                }
            }

            // 确保我们下一次的发送至少间隔 1 秒
            let send_next_query_interval_sec =
                dht_ctx_settings(self.ctx_index).send_next_query_interval_sec;
            tokio::time::sleep(Duration::from_secs(send_next_query_interval_sec)).await;
        }

        // log::debug!(target: "yiilian_dht::transaction::find_node", "nearest nodes count: {}", buckets.len());

        let nodes: Vec<Node> = buckets
            .get_nearest_nodes(&target_id, None)
            .into_iter()
            .map(|node| node.clone())
            .collect();

        nodes
    }

    /// Use the DHT to retrieve peers for the given info_hash.
    ///
    /// Returns the all the results so far after `timeout` has elapsed
    /// or the operation stops making progress (whichever happens first).
    pub(crate) async fn get_peers(
        &self,
        info_hash: Id,
        quick_mode: bool,
    ) -> Result<GetPeersResult, Error> {
        let mut unique_peers = HashSet::new();
        let mut responders = HashSet::new();
        let local_id = except_result!(
            dht_ctx_state(self.ctx_index).read(),
            "dht_ctx_state.read() failed"
        )
        .get_local_id();
        let bucket_size = dht_ctx_settings(self.ctx_index).bucket_size;
        let read_only = dht_ctx_settings(self.ctx_index).read_only;
        let send_query_timeout_sec = dht_ctx_settings(self.ctx_index).send_query_timeout_sec;

        let mut buckets = Buckets::new(bucket_size, local_id);

        // Hack to aid in bootstrapping
        // self.find_node(info_hash).await?;

        let mut best_ids = Vec::new();
        loop {
            // Populate our buckets with the main buckets from the DHT
            // 从路由表中获取所有的 node
            let all_verifyied = except_result!(
                dht_ctx_routing_tbl(self.ctx_index).lock(),
                "dht_ctx_routing_tbl.lock() failed"
            )
            .get_all_verified();

            for item in all_verifyied {
                if !buckets.contains(&item.id)
                    && !except_result!(
                        dht_ctx_routing_tbl(self.ctx_index).lock(),
                        "dht_ctx_routing_tbl.lock() failed"
                    )
                    .block_list
                    .contains(item.address.ip(), item.address.port())
                {
                    buckets.add(item, None).ok();
                }
            }
            // 在 buckets 中找到离 target_id 最近的节点，如果没找到任何节点，则稍后再尝试
            let nearest = buckets.get_nearest_nodes(&info_hash, None);

            if nearest.is_empty() {
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }

            let best_ids_current: Vec<Id> = nearest.iter().map(|node| node.id.clone()).collect();

            if best_ids == best_ids_current {
                // 直到找不到更近的节点，则退出循环
                break;
            }
            best_ids = best_ids_current;

            // 发送 get_peers 请求给这些节点
            let sender_id = local_id.clone();

            let todos = futures::stream::FuturesUnordered::new();
            for node in nearest {
                let get_peers_query = (
                    Query::GetPeers(GetPeers {
                        t: TransactionId::from_random(),
                        v: None,
                        ip: None,
                        ro: if read_only { Some(1) } else { None },
                        id: sender_id.clone(),
                        info_hash: info_hash.clone(),
                    }),
                    node.clone(),
                );
                todos.push(get_peers_query);
            }

            // Send get_peers to nearest nodes, handle their responses
            for (query, dest_node) in todos {
                let request_result = dht_ctx_trans_mgr(self.ctx_index)
                    .send_query(
                        query,
                        &dest_node.address,
                        Some(dest_node.id),
                        Some(Duration::from_secs(send_query_timeout_sec)),
                    )
                    .await;

                match request_result {
                    Ok(reply) => match reply {
                        Reply::GetPeers(val) => {
                            responders.insert(GetPeersResponder::new(dest_node.clone(), val.token));

                            if val.nodes.len() > 0 {
                                log::trace!(
                                    target: "yiilian_dht::transaction::get_peers",
                                    "[{}] Address {:?} got {} nodes",
                                    self.local_addr.port(), dest_node.address, val.nodes.len()
                                );

                                for node in val.nodes {
                                    // 将 find_node 返回的 nodes 加入到 routing_table 中
                                    let id_is_valid = {
                                        node.id.is_valid_for_ip(
                                            &node.address.ip(),
                                            &except_result!(
                                                dht_ctx_routing_tbl(self.ctx_index).lock(),
                                                "dht_ctx_routing_tbl.lock() failed"
                                            )
                                            .white_list,
                                        )
                                    };

                                    if id_is_valid && node.address.port() > 0
                                    // && !in_block_list
                                    {
                                        if quick_mode {
                                            // 方法1： send ping no wait
                                            let node_id = node.id;
                                            let node_addr = node.address;
                                            self.ping_no_wait(node_addr, Some(node_id)).await.ok();
                                        } else {
                                            // 方法2： 将获取的 nodes 加入到未验证 buckets 中
                                            if let Err(e) = except_result!(
                                                dht_ctx_routing_tbl(self.ctx_index).lock(),
                                                "dht_ctx_routing_tbl.lock() failed"
                                            )
                                            .add_or_update(node.clone(), false)
                                            {
                                                log::trace!(
                                                    target: "yiilian_dht::transaction::get_peers",
                                                    "[{}] Add node {:?} to buckets failed, error: {}",
                                                    self.local_addr.port(), node, e
                                                );
                                            }
                                        }
                                    }

                                    let in_block_list = except_result!(
                                        dht_ctx_routing_tbl(self.ctx_index).lock(),
                                        "dht_ctx_routing_tbl.lock() failed"
                                    )
                                    .is_blocked(&node.address);

                                    if !buckets.contains(&node.id) && !in_block_list {
                                        log::trace!(
                                            target: "yiilian_dht::transaction::get_peers",
                                            "[{}] Node (id: {:?}, {:?}) is a candidate for buckets",
                                            self.local_addr.port(), node.id, node.address
                                        );
                                        buckets.add(node.clone(), None).ok();
                                    }
                                }
                            }

                            if val.values.len() > 0 {
                                for peer in val.values {
                                    unique_peers.insert(peer);
                                }
                            }
                        }
                        _ => {
                            log::trace!(
                                target: "yiilian_dht::transaction::get_peers",
                                "[{}] Address {:?} got wrong packet type back: {:?}",
                                self.local_addr.port(), dest_node.address, reply
                            );

                            buckets.remove(&dest_node.id);
                            let reply_error_block_duration_sec =
                                dht_ctx_settings(self.ctx_index).reply_error_block_duration_sec;
                            except_result!(
                                dht_ctx_routing_tbl(self.ctx_index).lock(),
                                "dht_ctx_routing_tbl.lock() failed"
                            )
                            .add_block_list(
                                dest_node.address,
                                Some(dest_node.id),
                                Some(Duration::from_secs(reply_error_block_duration_sec)),
                            );
                        }
                    },
                    Err(error) => {
                        // 已在 send_query() 中加入了黑名单
                        buckets.remove(&dest_node.id);
                        log::trace!(
                            target: "yiilian_dht::transaction::get_peers",
                            "[{}] {:?} get_peers error: {}",
                            self.local_addr.port(), dest_node.address, error
                        );
                    }
                }
            }

            // 确保我们下一次的发送至少间隔 1 秒
            let send_next_query_interval_sec =
                dht_ctx_settings(self.ctx_index).send_next_query_interval_sec;
            tokio::time::sleep(Duration::from_secs(send_next_query_interval_sec)).await;
        }

        Ok(GetPeersResult::new(
            info_hash,
            unique_peers.into_iter().collect(),
            responders.into_iter().collect(),
        ))
    }

    /// Announce that you are a peer for a specific info_hash, returning the nodes
    /// that were successfully announced to.
    ///
    /// # Arguments
    /// * `info_hash` - Id of the torrent
    /// * `port` - optional port that other peers should use to contact your peer.
    /// If omitted, `implied_port` will be set true on the announce messages and
    /// * `timeout` - the maximum amount of time that will be spent searching for
    /// peers close to `info_hash` before announcing to them. This means that this
    /// function can actually take a bit longer than `timeout`, since it will take
    /// a moment after `timeout` has elapsed to announce to the nodes.
    pub(crate) async fn announce_peer(
        &self,
        info_hash: Id,
        port: Option<u16>,
    ) -> Result<Vec<Node>, Error> {
        let mut to_ret = Vec::new();

        // Figure out which nodes we want to announce to
        let get_peers_result = self.get_peers(info_hash.clone(), false).await?;

        log::trace!(
            target:"yiilian_dht::transaction::announce_peer",
            "[{}] {} nodes responded to get_peers",
            self.local_addr.port(), get_peers_result.responders().len()
        );

        // Prepare to send packets to the nearest 8 node
        let todos = futures::stream::FuturesUnordered::new();
        let bucket_size = dht_ctx_settings(self.ctx_index).bucket_size;
        for responder in get_peers_result.responders().into_iter().take(bucket_size) {
            let read_only = dht_ctx_settings(self.ctx_index).read_only;
            let announce_peer = (
                Query::AnnouncePeer(AnnouncePeer {
                    t: TransactionId::from_random(),
                    v: None,
                    ip: None,
                    ro: if read_only { Some(1) } else { None },
                    id: responder.node().id.clone(),
                    info_hash: info_hash.clone(),
                    implied_port: if let Some(_) = port { Some(0) } else { Some(1) },
                    port: port.unwrap_or(0),
                    token: responder.token().to_owned(),
                }),
                responder.node().to_owned(),
            );

            todos.push(announce_peer);
        }

        // Execute the futures, handle their results
        let send_query_timeout_sec = dht_ctx_settings(self.ctx_index).send_query_timeout_sec;
        for (query, dest_node) in todos {
            let request_result = dht_ctx_trans_mgr(self.ctx_index)
                .send_query(
                    query,
                    &dest_node.address,
                    Some(dest_node.id),
                    Some(Duration::from_secs(send_query_timeout_sec)),
                )
                .await;

            match request_result {
                Ok(reply) => match reply {
                    Reply::PingOrAnnounce(_) => {
                        to_ret.push(dest_node);
                    }
                    _ => {
                        log::trace!(
                            target: "yiilian_dht::transaction::announce_peer",
                            "[{}] Got wrong packet type back: {:?}",
                            self.local_addr.port(), reply
                        )
                    }
                },
                Err(e) => {
                    log::debug!(
                        target: "yiilian_dht::transaction::announce_peer",
                        "[{}] Error sending announce_peer: {}",
                        self.local_addr.port(), e
                    )
                }
            }
        }

        Ok(to_ret)
    }

    /// 每 10 秒清除一次创建时间在 10 秒前的请求事务
    pub async fn request_cleanup(&self) -> Result<(), Error> {
        let transaction_cleanup_interval_sec =
            dht_ctx_settings(self.ctx_index).transaction_cleanup_interval_sec;
        let mut interval = interval(Duration::from_secs(transaction_cleanup_interval_sec));

        loop {
            interval.tick().await;
            self.prune_older_than(Duration::from_secs(transaction_cleanup_interval_sec));
        }
    }
}
