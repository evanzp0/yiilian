use std::{fs::File, io::Write, net::SocketAddr};

use rand::thread_rng;
use yiilian_core::{common::error::Error, data::Encode};

use yiilian_bt::peer_wire::PeerWire;
use yiilian_dht::common::Id;

#[tokio::main]
async fn main() {
    let target_address: SocketAddr = "192.168.31.6:15000".parse().unwrap();
    let info_hash_str = "FA84A39C18D5960B0272D3E1D2A7900FB09F5EB3";
    let info_hash = hex::decode(info_hash_str)
        .map_err(|hex_err| Error::new_id(Some(hex_err.into()), None))
        .unwrap();

    let local_peer_id = Id::from_random(&mut thread_rng()).get_bytes();

    println!("connected");

    let peer_wire = PeerWire::new();
    let info = peer_wire
        .fetch_info(target_address, &info_hash, &local_peer_id)
        .await
        .unwrap();

    let torrent = info.encode();

    match std::fs::create_dir_all("./torrent/") {
        Ok(_) => {
            let mut f = File::create("./torrent/".to_string() + info_hash_str + ".torrent")
                .expect("File::create() node file failed");

            f.write_all(&torrent).expect("f.write_all() nodes failed");
        }
        Err(e) => {
            println!("{:?}", e);
        }
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
