use std::net::SocketAddr;

use rand::thread_rng;
use yiilian_core::net::tcp::{read_bt_handshake, send_bt_handshake};
use yiilian_dht::common::Id;

#[tokio::main]
async fn main() {
    let local_id = Id::from_random(&mut thread_rng());

    let target_addr: SocketAddr = "0.0.0.0:18080".parse().unwrap();
    let info_hash: [u8; 20] = {
        let info_hash_str = "98F09643766D5D561E986EFEB5BBA4F6BE98517E";

        let h = hex::decode(info_hash_str).unwrap();
        h.try_into().unwrap()
    };

    let mut stream = tokio::net::TcpStream::connect(target_addr).await.unwrap();

    send_bt_handshake(&mut stream, &info_hash, &local_id.to_vec()).await.unwrap();

    let handshake = read_bt_handshake(&mut stream).await.unwrap();

    println!("handshake: {:?}", handshake);
}
