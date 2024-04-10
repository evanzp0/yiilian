use std::fs;
use std::time::{Duration, Instant};

use hex::ToHex;
use yiilian_core::common::util::setup_log4rs_from_file;
use yiilian_core::common::working_dir::WorkingDir;
use yiilian_core::common::{error::Error, shutdown::create_shutdown};

use yiilian_dl::bt::common::BtConfig;
use yiilian_dl::bt::bt_downloader::BtDownloader;
use yiilian_dl::bt::common::DhtConfig;

#[tokio::main]
async fn main() {
    let wd = WorkingDir::new();
    let log4rs_path = wd.get_path_by_entry("log4rs.yml");
    setup_log4rs_from_file(&log4rs_path.unwrap());

    // let info_hash_str = "FA84A471E92F9DE5B4F2404E5535FCBA639DA8A0";
    // let info_hash_str = "5D238FCCC41203BD121080A0CF9C7788C8237A5A";
    let info_hash_str = "73985E7043186CCEB1BA8DCF1AFCBE26673C4D3A";

    let info_hash: [u8; 20] = {
        let h = hex::decode(info_hash_str)
        .map_err(|hex_err| Error::new_id(Some(hex_err.into()), None))
        .unwrap();
        h.try_into().unwrap()
    };

    println!("connected");

    let dht_config = DhtConfig { 
        routers: Some(vec![
            "87.98.162.88:6881".to_owned(),
            // "192.168.31.8:15000".to_owned(),
        ]),
        block_ips: None, 
        port: 20001, 
        workers: Some(1000), 
        firewall: None,
    };

    let bt_config = BtConfig::new(dht_config, 10800);
    let download_dir = {
        let mut d = home::home_dir().unwrap();
        d.push(".yiilian/dl/");

        fs::create_dir_all(d.clone())
            .map_err(|error| Error::new_file(Some(error.into()), None)).unwrap();
        d
    };
    println!("download_dir: {:?}", download_dir);

    let (mut shutdown_tx, shutdown_rx) = create_shutdown();

    let bt_downloader = BtDownloader::new(&bt_config, download_dir, shutdown_rx).unwrap();

    tokio::select! {
        _ = bt_downloader.run_loop() => (),
        _ = tokio::signal::ctrl_c() => {
            drop(bt_downloader);
            shutdown_tx.shutdown().await;

            println!("\nCtrl + c shutdown");
        },
        _ = async {
            // let target: SocketAddr = "192.168.31.8:15000".parse().unwrap();
            let timeout_sec = Duration::from_secs(3 * 60);
            let instant = Instant::now();
            let info_str: String =  info_hash.encode_hex();
            let mut blocked_addrs = vec![];

            loop {
                match bt_downloader.download_meta(&info_hash, &mut blocked_addrs, false).await {
                    Ok(_) => {
                        println!("{} is downloaded", info_str);
                        break;
                    },
                    Err(_) => {
                        if instant.elapsed() >= timeout_sec {
                            println!("{} is not founded", info_str);
                            break;
                        } else {
                            tokio::time::sleep(Duration::from_secs(1)).await
                        }
                    },
                }
            }

        } => (),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bytes::Bytes;
    use yiilian_core::data::{BencodeData, Encode};

    #[test]
    fn test() {
        let mut m = BencodeData::Map(BTreeMap::new());

        let mut m2: BTreeMap<Bytes, BencodeData> = BTreeMap::new();
        m2.insert(b"info"[..].into(), m);
        let a = BencodeData::Map(m2);

        println!("{:?}", a.encode());
    }
}
