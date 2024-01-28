use std::{net::SocketAddr, time::Duration};

use yiilian_core::{common::error::Error, data::{Request, Response}, service::Service};

use crate::{
    common::context::{dht_ctx_routing_tbl, dht_ctx_settings, dht_ctx_trans_mgr}, data::body::{BodyKind, KrpcBody, Query}, except_result, routing_table::Node
};

#[derive(Clone)]
pub struct RouterService {
    local_addr: SocketAddr,
}

impl RouterService {
    pub fn new(local_addr: SocketAddr) -> Self {
        RouterService { local_addr }
    }
}

impl Service<Request<KrpcBody>>  for RouterService {
    type Response = Response<KrpcBody>;

    type Error = Error;

    async fn call(&self, req: Request<KrpcBody>) -> Result<Self::Response, Self::Error> {
        let ctx_index = self.local_addr.port();
        let req_body = req.body.get_kind();

        let res = match req_body {
            BodyKind::Query(query) => {
                let read_only = dht_ctx_settings(ctx_index).read_only;
                
                if !read_only {
                    let sender_id = query.get_sender_id();

                    let is_id_valid = {
                        sender_id.is_valid_for_ip(
                            &req.remote_addr.ip(),
                            &except_result!(dht_ctx_routing_tbl(ctx_index).lock(), "Lock context routing_table failed").white_list,
                        )
                    };

                    let read_only = query.is_read_only();

                    // 有效，且对方不是 readonly （允许加入到我方的路由表的未验证 bucket 中）
                    if is_id_valid && !read_only {
                        except_result!(dht_ctx_routing_tbl(ctx_index).lock(), "Lock context routing_table failed")
                            .add_or_update(Node::new(sender_id, req.remote_addr.clone()), false)?;
                    }

                    let (reply, _) = match query {
                        Query::Ping(query) => {
                            dht_ctx_trans_mgr(ctx_index)
                                .handle_ping(query, &req.remote_addr)
                                .await?
                        }
                        Query::FindNode(query) => {
                            dht_ctx_trans_mgr(ctx_index)
                                .handle_find_node(query, &req.remote_addr)
                                .await?
                        }
                        Query::GetPeers(query) => {
                            dht_ctx_trans_mgr(ctx_index)
                                .handle_get_peers(query, &req.remote_addr)
                                .await?
                        }
                        Query::AnnouncePeer(query) => {
                            dht_ctx_trans_mgr(ctx_index)
                                .handle_announce_peer(query, &req.remote_addr)
                                .await?
                        }
                    };

                    let res_body = KrpcBody::new(BodyKind::Reply(reply));
                    let res = Response::new(res_body, req.remote_addr, req.local_addr);

                    res
                } else {
                    Response::new(KrpcBody::new(BodyKind::Empty), req.remote_addr, req.local_addr)
                }
            },
            BodyKind::Reply(reply) => {
                dht_ctx_trans_mgr(ctx_index)
                    .handle_reply(reply, &req.remote_addr)
                    .await?;
                Response::new(KrpcBody::new(BodyKind::Empty), req.remote_addr, req.local_addr)
            },
            BodyKind::RError(_) => {
                let reply_error_block_duration_sec = dht_ctx_settings(ctx_index).reply_error_block_duration_sec;
                except_result!(dht_ctx_routing_tbl(ctx_index).lock(), "Lock context routing_table failed")
                    .add_block_list(
                        req.remote_addr,
                        None,
                        Some(Duration::from_secs(reply_error_block_duration_sec)),
                    );
                    Response::new(KrpcBody::new(BodyKind::Empty), req.remote_addr, req.local_addr)
            },
            BodyKind::Empty => { Response::new(KrpcBody::new(BodyKind::Empty), req.remote_addr, req.local_addr) },
        };

        Ok(res)
    }
}
