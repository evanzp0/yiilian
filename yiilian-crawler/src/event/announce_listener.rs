use std::{
    fs::File,
    io::{Read, Write},
    sync::{Arc, RwLock},
};

use bloomfilter::Bloom;
use tokio::sync::broadcast::{error::RecvError, Receiver};
use yiilian_core::{
    common::{error::Error, shutdown::ShutdownReceiver, util::hash_it},
    data::Request,
    except_result,
};
use yiilian_dht::data::body::{BodyKind, KrpcBody, Query};

const BLOOM_STATE_FILE: &str = "bloom_state.dat";

#[derive(Debug)]
pub struct RecvAnnounceListener<T> {
    bloom: Arc<RwLock<Bloom<u64>>>,
    rx: Receiver<T>,
}

impl RecvAnnounceListener<Request<KrpcBody>> {
    pub fn new(rx: Receiver<Request<KrpcBody>>, shutdown: ShutdownReceiver) -> Self {
        let bloom = {
            match load_bloom() {
                Ok(bloom) => bloom,
                Err(_) => Arc::new(RwLock::new(Bloom::new_for_fp_rate(100_000_000, 0.001))),
            }
        };

        let bloom_for_save = bloom.clone();
        tokio::spawn(async move {
            log::trace!(target: "yiilian_cli::announce_listener", "Task '{}' starting up", "watch_shutdown");
            tokio::select! {
                _ = shutdown.watch() => {
                    save_bloom(bloom_for_save).await;
                },
            }
        });

        RecvAnnounceListener { rx, bloom }
    }

    pub async fn listen(&mut self) {
        loop {
            let rst = self.rx.recv().await;
            match rst {
                Ok(req) => {
                    match req.body.get_kind() {
                        BodyKind::Query(Query::AnnouncePeer(val)) => {
                            let bloom_val = hex::encode(val.info_hash.get_bytes());
                            let bloom_val = hash_it(bloom_val);
                            let chk_rst = except_result!(self.bloom.read(), "bloom.read() error")
                                .check(&bloom_val);

                            if !chk_rst {
                                // 如果没命中，则加入到布隆过滤其中，并输出到日志
                                except_result!(self.bloom.write(), "bloom.write() error")
                                    .set(&bloom_val);
                                log::info!(
                                    target: "yiilian_crawler::event::announce_listener",
                                    "recv announce: {:?} {:?}",
                                    val.info_hash,
                                    req.remote_addr
                                );
                            }
                        }
                        _ => (),
                    }
                }
                Err(error) => match error {
                    RecvError::Closed => {
                        println!("Send closed");
                        break;
                    }
                    RecvError::Lagged(_) => (),
                },
            }
        }
    }
}

/// save nodes to file
pub async fn save_bloom(bloom: Arc<RwLock<Bloom<u64>>>) {
    let mut f = File::create(&BLOOM_STATE_FILE).unwrap();
    let encoded: Vec<u8> = bincode::serialize(&*bloom).unwrap();

    f.write_all(&encoded).unwrap();
}

pub fn load_bloom() -> Result<Arc<RwLock<Bloom<u64>>>, Error> {
    match File::open(&BLOOM_STATE_FILE) {
        Ok(mut f) => {
            let mut buf: Vec<u8> = Vec::new();
            match f.read_to_end(&mut buf) {
                Ok(_) => {
                    let bloom: RwLock<Bloom<u64>> = bincode::deserialize(&buf[..]).unwrap();
                    let bloom = Arc::new(bloom);
                    Ok(bloom)
                }
                Err(e) => Err(Error::new_file(
                    Some(e.into()),
                    Some("file read failed in load_bloom()".to_owned()),
                ))?,
            }
        }
        Err(e) => Err(Error::new_file(
            Some(e.into()),
            Some("open file failed in load_bloom() ".to_owned()),
        ))?,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};

    use bloomfilter::Bloom;
    use yiilian_core::common::util::hash_it;

    use crate::event::announce_listener::save_bloom;

    #[test]
    fn test_bloom() {
        let bloom_val = hex::encode("abc");
        let bloom_val = hash_it(bloom_val);
        let mut bloom: Bloom<u64> = Bloom::new_for_fp_rate(100_000_000, 0.001);
        bloom.set(&bloom_val);

        assert_eq!(true, bloom.check(&bloom_val));
        assert_eq!(false, bloom.check(&1))
    }

    #[tokio::test]
    async fn test_serde() {
        let bloom_val = hex::encode("abc");
        let bloom_val = hash_it(bloom_val);
        let mut bloom: Bloom<u64> = Bloom::new_for_fp_rate(100_000_000, 0.001);
        bloom.set(&bloom_val);

        let bloom = Arc::new(RwLock::new(bloom));

        let encoded: Vec<u8> = bincode::serialize(&*bloom).unwrap();
        let bloom: RwLock<Bloom<u64>> = bincode::deserialize(&encoded[..]).unwrap();
        let check = bloom.read().unwrap().check(&bloom_val);
        assert_eq!(true, check);
        let check = bloom.read().unwrap().check(&1);
        assert_eq!(false, check);

        let bloom = Arc::new(bloom);
        save_bloom(bloom).await;
    }
}