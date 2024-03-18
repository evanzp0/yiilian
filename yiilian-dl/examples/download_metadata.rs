use std::{env::home_dir, fs::File, io::Write, net::SocketAddr, path::Path};

use rand::thread_rng;
use yiilian_core::{common::{error::Error, shutdown::{self, create_shutdown}}, data::Encode};

use yiilian_dl::bt::{common::BtConfig, peer_wire::PeerWire};
use yiilian_dht::common::Id;
use yiilian_dl::bt::bt_downloader::BtDownloader;
use yiilian_dl::bt::common::DhtConfig;

#[tokio::main]
async fn main() {
    set_up_logging_from_file::<&str>(None);

    let target_address: SocketAddr = "192.168.31.6:15000".parse().unwrap();
    let info_hash_str = "FA84A39C18D5960B0272D3E1D2A7900FB09F5EB3";
    let info_hash = hex::decode(info_hash_str)
        .map_err(|hex_err| Error::new_id(Some(hex_err.into()), None))
        .unwrap();

    // let local_peer_id = Id::from_random(&mut thread_rng()).get_bytes();

    println!("connected");

    // let peer_wire = PeerWire::new();
    // let info = peer_wire
    //     .fetch_info(target_address, &info_hash, &local_peer_id)
    //     .await
    //     .unwrap();

    // let torrent = info.encode();

    // match std::fs::create_dir_all("./torrent/") {
    //     Ok(_) => {
    //         let mut f = File::create("./torrent/".to_string() + info_hash_str + ".torrent")
    //             .expect("File::create() node file failed");

    //         f.write_all(&torrent).expect("f.write_all() nodes failed");
    //     }
    //     Err(e) => {
    //         println!("{:?}", e);
    //     }
    // }

    let dht_config = DhtConfig { 
        routers: Some(vec![
            // "87.98.162.88:6881".to_owned(),
            "192.168.31.8:15000".to_owned(),
        ]),
        block_ips: None, 
        port: 20001, 
        workers: Some(1000), 
        firewall: None,
    };

    let bt_config = BtConfig::new(dht_config);
    let download_dir = {
        let mut d = home::home_dir().unwrap();
        d.push(".yillian/dl/");
        d
    };
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
            let target: SocketAddr = "192.168.31.8:15000".parse().unwrap();
            bt_downloader.download_meta_from_target(target, &info_hash.try_into().unwrap()).await.unwrap();
        } => (),
    }
}


fn set_up_logging_from_file<P: AsRef<Path>>(file_path: Option<&P>) {
    if let Some(file_path) = file_path {
        log4rs::init_file(file_path, Default::default()).unwrap();
    } else {
        log4rs::init_file("log4rs.yml", Default::default()).unwrap();
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
