use std::sync::Arc;

use tokio::sync::broadcast::{error::RecvError, Receiver};
use yiilian_core::data::Request;
use yiilian_dht::data::body::{BodyKind, KrpcBody, Query};
use yiilian_mq::{engine::Engine, message::in_message::InMessage};

use crate::info_message::{InfoMessage, MessageType};

#[derive(Debug)]
pub struct RecvAnnounceListener<T> {
    rx: Receiver<Arc<T>>,
    mq_engine: Arc<Engine>,
}

impl RecvAnnounceListener<Request<KrpcBody>> {
    pub fn new(rx: Receiver<Arc<Request<KrpcBody>>>, mq_engine: Arc<Engine>) -> Self {

        RecvAnnounceListener { 
            rx, 
            mq_engine,
        }
    }

    pub async fn listen(&mut self) {
        loop {
            let rst = self.rx.recv().await;
            match rst {
                Ok(req) => {
                    match req.body.get_kind() {
                        BodyKind::Query(Query::GetPeers(val)) => {
                            let info_hash = {
                                let tmp = &val.info_hash.get_bytes()[0..];
                                let info_hash: [u8; 20] = tmp
                                    .try_into()
                                    .expect("Decode info_hash error");

                                info_hash
                            };

                            let data = InfoMessage {
                                try_times: 1,
                                info_type: MessageType::Normal(info_hash),
                            };

                            log::debug!(target: "yiilian_crawler::event::announce_listener", "Send message: {:?}", data);

                            self.mq_engine.push_message("info_hash", InMessage(data.into())).ok();
                        }
                        BodyKind::Query(Query::AnnouncePeer(val)) => {
                            let remote_addr = {
                                let implied_port = val.implied_port.unwrap_or(0);

                                if implied_port == 0 {
                                    let mut remote_addr = req.remote_addr;
                                    remote_addr.set_port(val.port);

                                    remote_addr
                                } else {
                                    req.remote_addr
                                }
                            };

                            let info_hash = {
                                let tmp = &val.info_hash.get_bytes()[0..];
                                let info_hash: [u8; 20] = tmp
                                    .try_into()
                                    .expect("Decode info_hash error");

                                info_hash
                            };

                            let data = InfoMessage {
                                try_times: 1,
                                info_type: MessageType::AnnouncePeer {info_hash, remote_addr},
                            };

                            log::debug!(target: "yiilian_crawler::event::announce_listener", "Send message: {:?}", data);

                            self.mq_engine.push_message("info_hash", InMessage(data.into())).ok();
                        }
                        _ => (),
                    }
                }
                Err(error) => match error {
                    RecvError::Closed => {
                        log::debug!(target: "yiilian_crawler::event::announce_listener", "Send closed");
                        break;
                    }
                    RecvError::Lagged(_) => (),
                },
            }
        }
    }
}