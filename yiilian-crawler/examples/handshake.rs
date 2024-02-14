use std::net::SocketAddr;

use rand::thread_rng;
use tokio::net::TcpStream;
use yiilian_core::common::error::Error;
use yiilian_crawler::{data::frame::Handshake, net::tcp::{read_handshake, send_handshake}};
use yiilian_dht::common::Id;

#[tokio::main]
async fn main() {
    let peer_address: SocketAddr = "192.168.31.6:15000".parse().unwrap();
    let info_hash = "FA84A39C18D5960B0272D3E1D2A7900FB09F5EB3";
    let info_hash = hex::decode(info_hash)
        .map_err(|hex_err| Error::new_id(Some(hex_err.into()), None))
        .unwrap();

    let peer_id = Id::from_random(&mut thread_rng()).get_bytes();

    let mut stream = TcpStream::connect(peer_address)
        .await
        .unwrap();

    println!("connected");

    // 发送握手消息给对方
    send_handshake(&mut stream, &info_hash, &peer_id).await.unwrap();

    // 接收对方回复的握手消息
    let rst = read_handshake(&mut stream).await.unwrap();

    // 校验对方握手消息
    if !Handshake::verify(&rst) {
        println!("recv handshake is invalid");
        return
    }

    // 发送扩展握手协议

    todo!()
}
