use std::sync::Arc;

// use bloomfilter::Bloom;

use bytes::BytesMut;
use tokio::sync::broadcast::{error::RecvError, Receiver};
use yiilian_core::{common::util::sockaddr_to_bytes, data::Request};
use yiilian_dht::data::body::{BodyKind, KrpcBody, Query};
use yiilian_mq::{engine::Engine, message::in_message::InMessage};

// const BLOOM_STATE_FILE: &str = "bloom_state.dat";

#[derive(Debug)]
pub struct RecvAnnounceListener<T> {
    // bloom: Arc<RwLock<Bloom<u64>>>,
    rx: Receiver<Arc<T>>,
    mq_engine: Arc<Engine>,
}

impl RecvAnnounceListener<Request<KrpcBody>> {
    pub fn new(rx: Receiver<Arc<Request<KrpcBody>>>, mq_engine: Arc<Engine>) -> Self {
        // let bloom = {
        //     match load_bloom() {
        //         Ok(bloom) => bloom,
        //         Err(_) => Arc::new(RwLock::new(Bloom::new_for_fp_rate(100_000_000, 0.001))),
        //     }
        // };

        // let bloom_for_save = bloom.clone();
        // tokio::spawn(async move {
        //     log::trace!(target: "yiilian_cli::announce_listener", "Task '{}' starting up", "watch_shutdown");
        //     tokio::select! {
        //         _ = shutdown.watch() => {
        //             save_bloom(bloom_for_save).await;
        //         },
        //     }
        // });

        RecvAnnounceListener { 
            rx, 
            mq_engine,
            // bloom, 
        }
    }

    pub async fn listen(&mut self) {
        loop {
            let rst = self.rx.recv().await;
            match rst {
                Ok(req) => {
                    match req.body.get_kind() {
                        BodyKind::Query(Query::AnnouncePeer(val)) => {
                            // let bloom_val = hex::encode(val.info_hash.get_bytes());
                            // let bloom_val = hash_it(bloom_val);
                            // let chk_rst = self
                            //     .bloom
                            //     .read()
                            //     .expect_error("bloom.read() error")
                            //     .check(&bloom_val);

                            // if !chk_rst {
                            //     // 如果没命中，则加入到布隆过滤其中，并输出到日志
                            //     self.bloom
                            //         .write()
                            //         .expect_error("bloom.write() error")
                            //         .set(&bloom_val);

                            //     self.mq_engine.push_message("info_hash", InMessage(val.info_hash.get_bytes())).ok();
                            // }
                            
                            let target_address = {
                                let implied_port = val.implied_port.unwrap_or(0);

                                if implied_port == 0 {
                                    let mut remote_addr = req.remote_addr;
                                    remote_addr.set_port(val.port);

                                    remote_addr
                                } else {
                                    req.remote_addr
                                }
                            };

                            // info_hash + compact_socket_address
                            let mut msg_val: BytesMut = BytesMut::new();
                            msg_val.extend(val.info_hash.get_bytes());
                            msg_val.extend(sockaddr_to_bytes(&target_address));

                            self.mq_engine.push_message("info_hash", InMessage(msg_val.into())).ok();
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

// pub async fn save_bloom(bloom: Arc<RwLock<Bloom<u64>>>) {
//     let mut f = File::create(&BLOOM_STATE_FILE).expect_error("file create BLOOM_STATE_FILE failed");
//     let encoded: Vec<u8> =
//         bincode::serialize(&*bloom).expect_error("bincode::serialize BLOOM_STATE_FILE failed");

//     f.write_all(&encoded)
//         .expect_error("write_all BLOOM_STATE_FILE failed");
// }

// pub fn load_bloom() -> Result<Arc<RwLock<Bloom<u64>>>, Error> {
//     match File::open(&BLOOM_STATE_FILE) {
//         Ok(mut f) => {
//             let mut buf: Vec<u8> = Vec::new();
//             match f.read_to_end(&mut buf) {
//                 Ok(_) => {
//                     let bloom: RwLock<Bloom<u64>> = bincode::deserialize(&buf[..])
//                         .expect_error("bincode::deserialize BLOOM_STATE_FILE failed");
//                     let bloom = Arc::new(bloom);
//                     Ok(bloom)
//                 }
//                 Err(e) => Err(Error::new_file(
//                     Some(e.into()),
//                     Some("file read failed in load_bloom()".to_owned()),
//                 ))?,
//             }
//         }
//         Err(e) => Err(Error::new_file(
//             Some(e.into()),
//             Some("open file failed in load_bloom() ".to_owned()),
//         ))?,
//     }
// }

// #[cfg(test)]
// mod tests {
//     use std::sync::{Arc, RwLock};

//     use bloomfilter::Bloom;
//     use yiilian_core::common::util::hash_it;

//     use crate::event::announce_listener::save_bloom;

//     #[test]
//     fn test_bloom() {
//         let bloom_val = hex::encode("abc");
//         let bloom_val = hash_it(bloom_val);
//         let mut bloom: Bloom<u64> = Bloom::new_for_fp_rate(100_000_000, 0.001);
//         bloom.set(&bloom_val);

//         assert_eq!(true, bloom.check(&bloom_val));
//         assert_eq!(false, bloom.check(&1))
//     }

//     #[tokio::test]
//     async fn test_serde() {
//         let bloom_val = hex::encode("abc");
//         let bloom_val = hash_it(bloom_val);
//         let mut bloom: Bloom<u64> = Bloom::new_for_fp_rate(100_000_000, 0.001);
//         bloom.set(&bloom_val);

//         let bloom = Arc::new(RwLock::new(bloom));

//         let encoded: Vec<u8> = bincode::serialize(&*bloom).unwrap();
//         let bloom: RwLock<Bloom<u64>> = bincode::deserialize(&encoded[..]).unwrap();
//         let check = bloom.read().unwrap().check(&bloom_val);
//         assert_eq!(true, check);
//         let check = bloom.read().unwrap().check(&1);
//         assert_eq!(false, check);

//         let bloom = Arc::new(bloom);
//         save_bloom(bloom).await;
//     }
// }
