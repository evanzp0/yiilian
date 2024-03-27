use std::{
    fs::{self, File},
    io::{Read, Write},
    net::SocketAddr,
    path::Path,
    sync::{Arc, RwLock},
    time::Duration,
};

use bloomfilter::Bloom;
use futures::future::join_all;

use hex::ToHex;
// use hex::ToHex;
use tokio::{
    net::TcpListener,
    signal::unix::SignalKind,
    sync::broadcast::{self, Sender},
};
use yiilian_core::{
    common::{
        error::Error,
        shutdown::{create_shutdown, ShutdownReceiver},
        util::hash_it,
        // util::{bytes_to_sockaddr,
        //     hash_it
        // },
    },
    data::Request,
    net::tcp::{read_bt_handshake, send_bt_handshake},
    service::{EventLayer, FirewallLayer},
};

use yiilian_dht::{
    common::SettingsBuilder,
    data::body::KrpcBody,
    dht::{Dht, DhtBuilder, DhtMode},
    service::KrpcService,
};
use yiilian_dl::bt::bt_downloader::BtDownloader;
use yiilian_mq::engine::Engine;

use yiilian_crawler::event::RecvAnnounceListener;
use yiilian_crawler::{
    common::{Config, DEFAULT_CONFIG_FILE},
    info_message::{InfoMessage, MessageType},
};

const BLOOM_STATE_FILE: &str = "bloom_state.dat";

#[tokio::main]
async fn main() {
    set_up_logging_from_file::<&str>(None);
    let config = Config::from_file(DEFAULT_CONFIG_FILE);
    let (mut shutdown_tx, shutdown_rx) = create_shutdown();
    let (tx, rx) = broadcast::channel(1024);
    let dht_list = create_dht_list(&config, shutdown_rx.clone(), tx).unwrap();

    let mq_engine = {
        let mut engine = Engine::new(shutdown_rx.clone()).expect("create mq engine");
        engine
            .open_topic("info_hash")
            .expect("open info_hash topic");

        Arc::new(engine)
    };

    let mut announce_listener = RecvAnnounceListener::new(rx, mq_engine.clone());

    let bloom = {
        match load_bloom() {
            Ok(bloom) => bloom,
            Err(_) => Arc::new(RwLock::new(Bloom::new_for_fp_rate(100_000_000, 0.001))),
        }
    };

    let download_dir = {
        let mut d = home::home_dir().unwrap();
        d.push(".yiilian/dl/");

        fs::create_dir_all(d.clone())
            .map_err(|error| Error::new_file(Some(error.into()), None))
            .unwrap();
        d
    };
    let bt_downloader = BtDownloader::new(&config.bt, download_dir, shutdown_rx.clone()).unwrap();

    let bm = bloom.clone();
    let strx = shutdown_rx.clone();
    tokio::spawn(async move {
        strx.watch().await;
        save_bloom(bm);
    });

    drop(shutdown_rx);

    let mut term_sig = tokio::signal::unix::signal(SignalKind::terminate()).unwrap();

    tokio::select! {
        _  = async {
            let mut futs = vec![];
            for dht in &dht_list {
                println!("Listening at: {:?}", dht.local_addr);
                futs.push(dht.run_loop());
            }

            join_all(futs).await;
        } => (),
        _ = announce_listener.listen() => (),
        _ = bt_downloader.run_loop() => (),
        // _ = download_meta(mq_engine, &bt_downloader, bloom.clone()) => (),
        _ = hook(&bt_downloader, bloom.clone(), config.hook_port) => (),
        _ = tokio::signal::ctrl_c() => {

            drop(dht_list);
            drop(bt_downloader);

            shutdown_tx.shutdown().await;

            println!("\nCtrl + c shutdown");
        },
        _ = term_sig.recv() => {
            drop(dht_list);
            drop(bt_downloader);

            shutdown_tx.shutdown().await;

            println!("\nShutdown");
        },
    };
}

async fn hook(bt_downloader: &BtDownloader, bloom: Arc<RwLock<Bloom<u64>>>, port: u16) -> Result<(), Error> {
    let bind_addr: SocketAddr = format!( "0.0.0.0:{port}").parse().expect("tcp bind error in hook");
    let listener = TcpListener::bind(bind_addr).await.map_err(|error| {
        Error::new_net(
            Some(error.into()),
            Some("tcp listen error in hook".to_owned()),
            Some(bind_addr),
        )
    })?;

    println!("Hooking at: {}", listener.local_addr().unwrap());

    while let Ok((mut stream, target_addr)) = listener.accept().await {

        log::trace!(target:"yiilian_crawler::main::hook", "Accept address: {:?}", target_addr);

        // 接收对方回复的握手消息
        let handshake = read_bt_handshake(&mut stream).await?;

        // 发送握手消息给对方
        send_bt_handshake(&mut stream, handshake.info_hash(), bt_downloader.local_id()).await?;

        let info_hash: [u8; 20] = {
            handshake.info_hash()[..]
                .try_into()
                .expect("Decode info_hash in handshake error")
        };
        let info_str: String = info_hash.encode_hex_upper();

        let bloom_val = hex::encode(info_hash);
        let bloom_val = hash_it(bloom_val);
        let chk_rst = bloom.read().expect("bloom.read() error").check(&bloom_val);

        if !chk_rst {
            match bt_downloader
                .download_meta_from_target(stream, &info_hash, true)
                .await
            {
                Ok(_) => {
                    // 如果没命中且成功下载，则加入到布隆过滤其中，并输出到日志
                    bloom.write().expect("bloom.write() error").set(&bloom_val);

                    log::debug!(target: "yiilian_crawler::main::hook", "{} is downloaded", info_str);
                }
                Err(error) => {
                    log::trace!(target: "yiilian_crawler::main::hook", "{} is failure: {}", info_str, error);
                }
            }
        }
    }

    Ok(())
}

async fn _download_meta(
    mq_engine: Arc<Engine>,
    _bt_downloader: &BtDownloader,
    _bloom: Arc<RwLock<Bloom<u64>>>,
) {
    loop {
        // let mut blocked_addrs = vec![];

        let msg_rst = mq_engine.poll_message("info_hash", "download_meta_client");
        if let Some(msg) = msg_rst {
            // todo! info_message and if not download then change message to normal and send into mq again

            let info_message: InfoMessage = {
                match msg.value().try_into() {
                    Ok(msg) => msg,
                    Err(error) => {
                        log::trace!(target: "yiilian_crawler::download_meta", "Decode info_message error: {:?} ", error);
                        continue;
                    }
                }
            };

            match info_message.info_type {
                MessageType::Normal(_info_hash) => {}
                MessageType::GetPeers {
                    info_hash: _,
                    remote_addr: _,
                } => {}
                MessageType::AnnouncePeer {
                    info_hash: _,
                    remote_addr: _,
                } => {}
            }

            todo!();

            // let info_hash: [u8; 20] = {
            //     let value = &msg.value()[0..20];
            //     match value.try_into() {
            //         Ok(value) => value,
            //         Err(_) => continue,
            //     }
            // };

            // let info_str: String = info_hash.encode_hex_upper();

            // let target_addr = {
            //     match bytes_to_sockaddr(&msg.value()[20..]) {
            //         Ok(value) => value,
            //         Err(_) => continue,
            //     }
            // };

            // if blocked_addrs.contains(&target_addr) {
            //     continue
            // }

            // log::trace!(target: "yiilian_crawler::main", "poll message infohash: {} , target: {} , offset : {}", info_str, target_addr, msg.offset());

            // let bloom_val = hex::encode(info_hash);
            // let bloom_val = hash_it(bloom_val);
            // let chk_rst = bloom.read().expect("bloom.read() error").check(&bloom_val);

            // if !chk_rst {
            //     match bt_downloader
            //         .download_meta_from_target(target_addr, &info_hash, &mut blocked_addrs)
            //         .await
            //     {
            //         Ok(_) => {
            //             // 如果没命中且成功下载，则加入到布隆过滤其中，并输出到日志
            //             bloom.write().expect("bloom.write() error").set(&bloom_val);

            //             log::debug!(target: "yiilian_crawler::main", "{} is downloaded", info_str);
            //         }
            //         Err(_) => {

            //             match bt_downloader
            //                 .download_meta(&info_hash, &mut blocked_addrs)
            //                 .await
            //             {
            //                 Ok(_) => {
            //                     // 如果没命中且成功下载，则加入到布隆过滤其中，并输出到日志
            //                     bloom.write().expect("bloom.write() error").set(&bloom_val);

            //                     log::debug!(target: "yiilian_crawler::main", "{} is downloaded", info_str);
            //                 }
            //                 Err(_) => {
            //                     log::trace!(target: "yiilian_crawler::main", "{} is not founded", info_str);
            //                 }
            //             }
            //         }
            //     }
            // }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

fn create_dht_list(
    config: &Config,
    shutdown_rx: ShutdownReceiver,
    tx: Sender<Arc<Request<KrpcBody>>>,
) -> Result<
    Vec<
        Dht<impl KrpcService<KrpcBody, ResBody = KrpcBody, Error = Error> + Clone + Send + 'static>,
    >,
    Error,
> {
    let mut dht_list = vec![];

    let ports = &config.dht_cluster.ports;
    let block_ips = config.get_dht_block_list();
    let workers = config.dht_cluster.workers;

    let settings = if let Some(routers) = &config.dht_cluster.routers {
        let mut st = SettingsBuilder::new().build();
        st.routers = routers.clone();
        Some(st)
    } else {
        None
    };

    let (firewall_max_trace, firewall_max_block) = {
        if let Some(firewall_config) = &config.dht_cluster.firewall {
            (
                firewall_config.max_trace.unwrap_or(500),
                firewall_config.max_block.unwrap_or(1000),
            )
        } else {
            (500, 1000)
        }
    };

    if ports.len() == 2 {
        let port_start = ports[0];
        let port_end = ports[1];
        for port in port_start..=port_end {
            let local_addr: SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();

            let dht = DhtBuilder::new(local_addr, shutdown_rx.clone(), workers)
                .block_list(block_ips.clone())
                .settings(settings.clone())
                .mode(DhtMode::Crawler(config.hook_port))
                .layer(FirewallLayer::new(
                    firewall_max_trace,
                    20,
                    firewall_max_block,
                    shutdown_rx.clone(),
                ))
                .layer(EventLayer::new(tx.clone()))
                .build()
                .unwrap();

            dht_list.push(dht);
        }
    } else {
        for port in ports {
            let local_addr: SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();

            let dht = DhtBuilder::new(local_addr, shutdown_rx.clone(), workers)
                .block_list(block_ips.clone())
                .settings(settings.clone())
                .mode(DhtMode::Crawler(config.hook_port))
                .layer(FirewallLayer::new(
                    10,
                    20,
                    firewall_max_block,
                    shutdown_rx.clone(),
                ))
                .layer(EventLayer::new(tx.clone()))
                .build()
                .unwrap();

            dht_list.push(dht);
        }
    }

    Ok(dht_list)
}

fn set_up_logging_from_file<P: AsRef<Path>>(file_path: Option<&P>) {
    if let Some(file_path) = file_path {
        log4rs::init_file(file_path, Default::default()).unwrap();
    } else {
        log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    }
}

pub fn save_bloom(bloom: Arc<RwLock<Bloom<u64>>>) {
    let mut f = File::create(&BLOOM_STATE_FILE).expect("file create BLOOM_STATE_FILE failed");
    let encoded: Vec<u8> =
        bincode::serialize(&*bloom).expect("bincode::serialize BLOOM_STATE_FILE failed");

    f.write_all(&encoded)
        .expect("write_all BLOOM_STATE_FILE failed");
}

pub fn load_bloom() -> Result<Arc<RwLock<Bloom<u64>>>, Error> {
    match File::open(&BLOOM_STATE_FILE) {
        Ok(mut f) => {
            let mut buf: Vec<u8> = Vec::new();
            match f.read_to_end(&mut buf) {
                Ok(_) => {
                    let bloom: RwLock<Bloom<u64>> = bincode::deserialize(&buf[..])
                        .expect("bincode::deserialize BLOOM_STATE_FILE failed");
                    Ok(Arc::new(bloom))
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
