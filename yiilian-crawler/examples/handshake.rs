use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use rand::thread_rng;
use tokio::net::UdpSocket;
use tokio::{io::AsyncWriteExt, net::TcpStream};
use yiilian_core::common::error::Error;
use yiilian_crawler::data::frame::{Handshake, MESSAGE_EXTENSION_ENABLE};
use yiilian_crawler::net::tcp::read_all;
use yiilian_dht::common::Id;

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

    // let mut stream = TcpStream::connect(peer_address)
    //     .await
    //     .unwrap();

    // stream
    //     .write_all(&hs)
    //     .await
    //     .map_err(|error| Error::new_net(Some(error.into()), None, Some(peer_address)))
    //     .unwrap();

    // stream.flush().await.unwrap();

    // let rst = read_all(&mut stream).await.unwrap();

    // println!("{:?}", rst);

    let std_sock =
        std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    std_sock
        .set_nonblocking(true)
        .unwrap();

    let socket = Arc::new(UdpSocket::from_std(std_sock).unwrap());

    yiilian_core::net::udp::send_to(&socket, &hs, peer_address).await.unwrap();

    let a = yiilian_core::net::udp::recv_from(&socket).await.unwrap();

    println!("{:?}", a);
}
