
use std::net::SocketAddr;

use bytes::Bytes;
use rand::thread_rng;
use tokio::{io::AsyncWriteExt, net::TcpStream};
use yiilian_core::common::error::Error;
use yiilian_crawler::data::frame::{Handshake, MESSAGE_EXTENSION_ENABLE};
use yiilian_dht::common::Id;
use yiilian_crawler::net::tcp::read_all;

#[tokio::main]
async fn main() {
    let peer_address: SocketAddr = "112.244.15.28:8134".parse().unwrap();
    let info_hash = "a93f35d9c1ba7557fb8fb7a59f36a051cc9c88a1";
    let info_hash = hex::decode(info_hash)
        .map_err(|hex_err| Error::new_id(Some(hex_err.into()), None))
        .unwrap();

    let peer_id = Id::from_random(&mut thread_rng()).get_bytes();
    let hs = Handshake::new(&MESSAGE_EXTENSION_ENABLE, &info_hash, &peer_id);
    let hs: Bytes = hs.into();

    let mut stream = TcpStream::connect(peer_address)
        .await
        .unwrap();
    
    // stream
    //     .write_all(&hs)
    //     .await
    //     .map_err(|error| Error::new_net(Some(error.into()), None, Some(peer_address)))
    //     .unwrap();

    // stream.flush().await.unwrap();

    // let rst = read_all(&mut stream).await.unwrap();

    // println!("{:?}", rst);
}
