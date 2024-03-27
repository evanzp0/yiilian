
use std::{net::SocketAddr, path::Path};

use rand::thread_rng;
use yiilian_core::common::error::Error;
use yiilian_dht::common::Id;
use yiilian_dl::bt::peer_wire::PeerWire;

#[tokio::main]
async fn main() {
    set_up_logging_from_file::<&str>(None);

    let peer_wire = PeerWire::new();
    let local_id = Id::from_random(&mut thread_rng());

    let target_addr: SocketAddr = "142.215.164.101:6882".parse().unwrap();
    let info_hash: [u8; 20] = {
        let info_hash_str = "98F09643766D5D561E986EFEB5BBA4F6BE98517E";

        let h = hex::decode(info_hash_str)
        .map_err(|hex_err| Error::new_id(Some(hex_err.into()), None))
        .unwrap();
        h.try_into().unwrap()
    };

    let stream = tokio::net::TcpStream::connect(target_addr).await.unwrap();

    match peer_wire
        .fetch_info(stream, &info_hash, &local_id.to_vec(), false)
        .await
    {
        Ok(info) => {
            println!("Ok: {:?}", info);
        },
        Err(error) => {
            println!("Error: {:?}", error);

        }
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
