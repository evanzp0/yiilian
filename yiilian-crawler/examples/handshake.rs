
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
    let peer_address: SocketAddr = "142.215.164.109:6882".parse().unwrap();
    let info_hash = "f7707928c2c3f3655c62ec3c7b0ef4dcd99ae3f3";
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
