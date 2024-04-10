use std::{
    fs::{self, File}, io::{Read, Write}, net::SocketAddr, path::PathBuf, sync::{Arc, Mutex, RwLock}, time::Duration
};

use bloomfilter::Bloom;
use futures::future::join_all;

use hex::ToHex;
use tantivy::{schema::Schema, Index};
use tokio::{
    net::TcpListener,
    signal::unix::SignalKind,
    sync::broadcast::{self, Sender},
    time::sleep,
};
use yiilian_core::{
    common::{
        error::Error,
        shutdown::{create_shutdown, ShutdownReceiver},
        util::{hash_it, setup_log4rs_from_file}, working_dir::WorkingDir,
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
use yiilian_index::{info_db_to_doc::InfoDbToDocBuilder, info_mq_to_db::InfoMqToDbBuilder};
use yiilian_mq::{
    engine::{self, Engine},
    message::in_message::InMessage,
    segment::LOG_DATA_SIZE,
};

use yiilian_crawler::event::RecvAnnounceListener;
use yiilian_crawler::{
    common::Config,
    info_message::{InfoMessage, MessageType},
};

const BLOOM_STATE_FILE: &str = "bloom_state.dat";
const HASH_TOPIC_NAME: &str = "info_hash";
const INDEX_TOPIC_NAME: &str = "info_index";
const CONFIG_FILE: &str = "yiilian-crawler.yml";
const LOG_CONFIG_FILE: &str = "log4rs.yml";
const RES_TEMPLATE_DB: &str = "res_template.db";

#[tokio::main]
async fn main() {
    let wd = WorkingDir::new();
    let log4rs_path = wd.get_path_by_entry(LOG_CONFIG_FILE);
    setup_log4rs_from_file(&log4rs_path.unwrap());

    let config_file = wd.get_path_by_entry(CONFIG_FILE).unwrap();

    let config = Config::from_file(config_file);
    let (mut shutdown_tx, shutdown_rx) = create_shutdown();
    let (tx, rx) = broadcast::channel(1024);
    let dht_list = create_dht_list(&config, shutdown_rx.clone(), tx, wd.home_dir()).unwrap();
    let mq_engine = {
        let mut engine = Engine::new(LOG_DATA_SIZE, wd.home_dir()).expect("create mq engine");
        engine
            .open_topic(HASH_TOPIC_NAME)
            .expect(&format!("open {} topic", HASH_TOPIC_NAME));
        engine
            .open_topic(INDEX_TOPIC_NAME)
            .expect(&format!("open {} topic", INDEX_TOPIC_NAME));

        Arc::new(Mutex::new(engine))
    };

    let mut announce_listener = RecvAnnounceListener::new(rx, mq_engine.clone());

    let bloom = {
        match load_bloom() {
            Ok(bloom) => bloom,
            Err(_) => Arc::new(RwLock::new(Bloom::new_for_fp_rate(100_000_000, 0.001))),
        }
    };

    let download_dir = {
        let mut d = wd.home_dir();
        d.push(".yiilian/dl/");

        fs::create_dir_all(d.clone())
            .map_err(|error| Error::new_file(Some(error.into()), None))
            .unwrap();
        d
    };
    let bt_downloader = BtDownloader::new(&config.bt, download_dir, shutdown_rx.clone(), wd.home_dir()).unwrap();

    let bm = bloom.clone();
    let strx = shutdown_rx.clone();
    let exec_dir = wd.exec_dir();
    tokio::spawn(async move {
        strx.watch().await;
        save_bloom(bm, exec_dir);
    });

    let db_uri = {
        let mut p = wd.home_dir();
        p.push(".yiilian/db/res.db");

        if !p.as_path().exists() {
            let mut db_dir = wd.home_dir();
            db_dir.push(".yiilian/db");
            fs::create_dir_all(db_dir).unwrap();

            let c_path = wd.get_path_by_entry(RES_TEMPLATE_DB).unwrap();

            fs::copy(c_path, p.clone()).unwrap();
        }

        let p = p.to_str().unwrap();
        p.to_owned()
    };

    let mut mq_db = InfoMqToDbBuilder::new()
        .db_uri(&db_uri)
        .await
        .mq_engine(mq_engine.clone())
        .build();

    let schema = Schema::builder().build();
    let index = Index::create_in_ram(schema.clone());
    let mut db_doc = InfoDbToDocBuilder::new()
        .db_uri(&db_uri)
        .await
        .index(index)
        .schema(schema)
        .build();

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
        _ = download_meta_by_msg(mq_engine.clone(), &bt_downloader, bloom.clone()) => (),
        _ = hook(&bt_downloader, bloom.clone(), config.bt.download_port, mq_engine.clone()) => (),
        _= mq_db.persist_loop() => (),
        _= db_doc.index_loop() => (),
        _ = async move {
            engine::purge_loop(mq_engine).await
        } => (),
        _ = tokio::signal::ctrl_c() => {

            drop(dht_list);
            drop(bt_downloader);
            drop(mq_db);
            drop(db_doc);

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

async fn hook(
    bt_downloader: &BtDownloader,
    bloom: Arc<RwLock<Bloom<u64>>>,
    port: u16,
    mq_engine: Arc<Mutex<Engine>>,
) {
    let bind_addr: SocketAddr = format!("0.0.0.0:{port}")
        .parse()
        .expect("tcp bind error in hook");
    let listener = TcpListener::bind(bind_addr)
        .await
        .expect("tcp listen error in hook");

    println!("Download at: {}", listener.local_addr().unwrap());

    loop {
        match listener.accept().await {
            Err(error) => {
                log::trace!(target: "yiilian_crawler::main::hook", "{:?}", error);
            }
            Ok((mut stream, target_addr)) => {
                log::trace!(target:"yiilian_crawler::main::hook", "Accept address: {:?}", target_addr);

                // 接收对方回复的握手消息
                let handshake = if let Ok(rst) = read_bt_handshake(&mut stream).await {
                    rst
                } else {
                    continue;
                };

                // 发送握手消息给对方
                if let Err(_) =
                    send_bt_handshake(&mut stream, handshake.info_hash(), bt_downloader.local_id())
                        .await
                {
                    continue;
                }

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
                        Ok(path) => {
                            // 如果没命中且成功下载，则加入到布隆过滤其中，并输出到日志
                            bloom.write().expect("bloom.write() error").set(&bloom_val);

                            log::debug!(target: "yiilian_crawler::main::hook", "{} is downloaded", info_str);

                            let path = match path.to_str() {
                                Some(p) => p.to_owned(),
                                None => continue,
                            };
                            let message = yiilian_mq::message::in_message::InMessage(path.into());
                            if let Err(error) = mq_engine
                                .lock()
                                .expect("lock mq_engin")
                                .push_message(INDEX_TOPIC_NAME, message)
                            {
                                log::trace!(target: "yiilian_crawler::main::hook", "push_message error: {}", error);
                            }
                        }
                        Err(error) => {
                            log::trace!(target: "yiilian_crawler::main::hook", "{}", error);
                        }
                    }
                }
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
}

async fn download_meta_by_msg(
    mq_engine: Arc<Mutex<Engine>>,
    bt_downloader: &BtDownloader,
    bloom: Arc<RwLock<Bloom<u64>>>,
) {
    loop {
        let msg_rst = mq_engine
            .lock()
            .expect("lock mq_engin")
            .poll_message(HASH_TOPIC_NAME, "download_meta_client");

        if let Some(msg) = msg_rst {
            log::trace!(target: "yiilian_crawler::main", "poll message offset : {}", msg.offset());

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
                MessageType::Normal(info_hash) => {
                    if info_message.try_times <= 0 {
                        continue;
                    }

                    let mut blocked_addrs = vec![];
                    let info_str: String = info_hash.encode_hex_upper();

                    match bt_downloader
                        .download_meta(&info_hash, &mut blocked_addrs, false)
                        .await
                    {
                        Ok(path) => {
                            let bloom_val = hex::encode(info_hash);
                            let bloom_val = hash_it(bloom_val);
                            let chk_rst =
                                bloom.read().expect("bloom.read() error").check(&bloom_val);

                            if !chk_rst {
                                // 如果没命中且成功下载，则加入到布隆过滤其中，并输出到日志
                                bloom.write().expect("bloom.write() error").set(&bloom_val);

                                log::debug!(target: "yiilian_crawler::main", "{} is downloaded", info_str);

                                let path = match path.to_str() {
                                    Some(p) => p.to_owned(),
                                    None => continue,
                                };
                                let message =
                                    yiilian_mq::message::in_message::InMessage(path.into());
                                if let Err(error) = mq_engine
                                    .lock()
                                    .expect("lock mq_engin")
                                    .push_message(INDEX_TOPIC_NAME, message)
                                {
                                    log::trace!(target: "yiilian_crawler::main::hook", "push_message error: {}", error);
                                }
                            }
                        }
                        Err(error) => {
                            log::trace!(target: "yiilian_crawler::main::download_meta_by_msg", "Resend message by error: {error}");

                            let msg_data = InfoMessage {
                                try_times: info_message.try_times - 1,
                                info_type: MessageType::Normal(info_hash),
                            };

                            mq_engine
                                .lock()
                                .expect("lock mq_engin")
                                .push_message(HASH_TOPIC_NAME, InMessage(msg_data.into()))
                                .ok();
                        }
                    }
                }
                MessageType::AnnouncePeer {
                    info_hash,
                    remote_addr,
                } => {
                    if info_message.try_times <= 0 {
                        continue;
                    }

                    let info_str: String = info_hash.encode_hex_upper();
                    let bloom_val = hex::encode(info_hash);
                    let bloom_val = hash_it(bloom_val);
                    let chk_rst = bloom.read().expect("bloom.read() error").check(&bloom_val);

                    let stream = if let Ok(s) = tokio::net::TcpStream::connect(remote_addr).await {
                        s
                    } else {
                        continue;
                    };

                    if !chk_rst {
                        match bt_downloader
                            .download_meta_from_target(stream, &info_hash, false)
                            .await
                        {
                            Ok(path) => {
                                // 如果没命中且成功下载，则加入到布隆过滤其中，并输出到日志
                                bloom.write().expect("bloom.write() error").set(&bloom_val);

                                log::debug!(target: "yiilian_crawler::main::download_meta_by_msg", "{} is downloaded", info_str);

                                let path = match path.to_str() {
                                    Some(p) => p.to_owned(),
                                    None => continue,
                                };
                                let message =
                                    yiilian_mq::message::in_message::InMessage(path.into());
                                if let Err(error) = mq_engine
                                    .lock()
                                    .expect("lock mq_engin")
                                    .push_message(INDEX_TOPIC_NAME, message)
                                {
                                    log::trace!(target: "yiilian_crawler::main::hook", "push_message error: {}", error);
                                }
                            }
                            Err(error) => {
                                log::trace!(target: "yiilian_crawler::main::download_meta_by_msg", "Resend message by error: {error}");

                                let msg_data = InfoMessage {
                                    try_times: info_message.try_times - 1,
                                    info_type: MessageType::Normal(info_hash),
                                };

                                mq_engine
                                    .lock()
                                    .expect("lock mq_engin")
                                    .push_message(HASH_TOPIC_NAME, InMessage(msg_data.into()))
                                    .ok();
                            }
                        }
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

fn create_dht_list(
    config: &Config,
    shutdown_rx: ShutdownReceiver,
    tx: Sender<Arc<Request<KrpcBody>>>,
    home_dir: PathBuf,
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

            let dht = DhtBuilder::new(local_addr, shutdown_rx.clone(), workers, home_dir.clone())
                .block_list(block_ips.clone())
                .settings(settings.clone())
                .mode(DhtMode::Crawler(config.bt.download_port))
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

            let dht = DhtBuilder::new(local_addr, shutdown_rx.clone(), workers, home_dir.clone())
                .block_list(block_ips.clone())
                .settings(settings.clone())
                .mode(DhtMode::Crawler(config.bt.download_port))
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


pub fn save_bloom(bloom: Arc<RwLock<Bloom<u64>>>, mut save_dir: PathBuf) {
    save_dir.push(BLOOM_STATE_FILE);
    let mut f = File::create(save_dir).expect("file create BLOOM_STATE_FILE failed");
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
