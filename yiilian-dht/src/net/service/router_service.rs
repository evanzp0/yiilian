use std::{sync::Arc, time::Duration};

use yiilian_core::{common::error::Error, data::{Request, Response}, service::Service};

use crate::{
    common::context::Context, data::body::{BodyKind, KrpcBody, Query}, routing_table::Node
};

pub struct RouterService {
    ctx: Arc<Context>,
}

impl RouterService {
    pub fn new(ctx: Arc<Context>) -> Self {
        RouterService { ctx }
    }
}

impl Service<Request<KrpcBody>>  for RouterService {
    type Response = Response<KrpcBody>;

    type Error = Error;

    async fn call(&self, req: Request<KrpcBody>) -> Result<Self::Response, Self::Error> {
        let req_body = req.body.get_kind();

        let res = match req_body {
            BodyKind::Query(query) => {
                if !self.ctx.settings().read_only {
                    let sender_id = query.get_sender_id();

                    let is_id_valid = {
                        sender_id.is_valid_for_ip(
                            &req.remote_addr.ip(),
                            &self.ctx.routing_table().lock().unwrap().white_list,
                        )
                    };

                    let read_only = query.is_read_only();

                    // 有效，且对方不是 readonly （允许加入到我方的路由表的未验证 bucket 中）
                    if is_id_valid && !read_only {
                        self.ctx.routing_table()
                            .lock()
                            .unwrap()
                            .add_or_update(Node::new(sender_id, req.remote_addr.clone()), false, self.ctx.clone())?;
                    }

                    let (reply, _) = match query {
                        Query::Ping(query) => {
                            self.ctx.transaction_manager()
                                .handle_ping(query, &req.remote_addr, self.ctx.clone())
                                .await?
                        }
                        Query::FindNode(query) => {
                            self.ctx.transaction_manager()
                                .handle_find_node(query, &req.remote_addr, self.ctx.clone())
                                .await?
                        }
                        Query::GetPeers(query) => {
                            self.ctx.transaction_manager()
                                .handle_get_peers(query, &req.remote_addr, self.ctx.clone())
                                .await?
                        }
                        Query::AnnouncePeer(query) => {
                            self.ctx.transaction_manager()
                                .handle_announce_peer(query, &req.remote_addr, self.ctx.clone())
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
                self.ctx.transaction_manager()
                    .handle_reply(reply, &req.remote_addr, self.ctx.clone())
                    .await?;
                Response::new(KrpcBody::new(BodyKind::Empty), req.remote_addr, req.local_addr)
            },
            BodyKind::RError(_) => {
                let reply_error_block_duration_sec = self.ctx.settings().reply_error_block_duration_sec;
                self.ctx.routing_table()
                    .lock()
                    .unwrap()
                    .add_block_list(
                        req.remote_addr,
                        None,
                        Some(Duration::from_secs(reply_error_block_duration_sec)),
                        self.ctx.clone(),
                    );
                    Response::new(KrpcBody::new(BodyKind::Empty), req.remote_addr, req.local_addr)
            },
            BodyKind::Empty => { Response::new(KrpcBody::new(BodyKind::Empty), req.remote_addr, req.local_addr) },
        };

        Ok(res)
    }
}
