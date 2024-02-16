use std::net::SocketAddr;

use bytes::{BufMut, BytesMut};
use rand::thread_rng;
use yiilian_core::{common::error::Error, data::decode};

use yiilian_crawler::peer_wire::PeerWire;
use yiilian_dht::common::Id;

#[tokio::main]
async fn main() {
    let peer_address: SocketAddr = "192.168.31.6:15000".parse().unwrap();
    let info_hash = "FA84A39C18D5960B0272D3E1D2A7900FB09F5EB3";
    let info_hash = hex::decode(info_hash)
        .map_err(|hex_err| Error::new_id(Some(hex_err.into()), None))
        .unwrap();

    let peer_id = Id::from_random(&mut thread_rng()).get_bytes();

    println!("connected");

    let peer_wire = PeerWire::new();
    let metadata = peer_wire.fetch_metdata(peer_address, &info_hash, &peer_id).await.unwrap();
    let mut info = BytesMut::new();
    info.put(&b"d4:info"[..]);
    info.extend(metadata);
    info.put(&b"e"[..]);

    let info = decode(&info).unwrap();

    let m = info.as_map().unwrap();
    let info = m.get(&b"info"[..]).unwrap().as_map().unwrap();
    let name = info.get(&b"name"[..]).unwrap();
    let piece_length = info.get(&b"piece length"[..]).unwrap();

    println!("{:?}, {}", name, piece_length);
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