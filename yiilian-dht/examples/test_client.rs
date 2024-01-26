
use std::{net::SocketAddr, sync::Arc};

use tokio::net::UdpSocket;
use yiilian_core::{common::error::Error, data::Request};
use yiilian_dht::{data::{body::{KrpcBody, Query, Reply}, ping::Ping, ping_announce_replay::PingOrAnnounceReply }, net::Client};


#[tokio::main]
async fn main() {
    let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let socket = Arc::new(build_socket(local_addr).unwrap());

    let client = Client::new(socket);

    let remote_addr: SocketAddr = "127.0.0.1:6578".parse().unwrap();
    let ping = Ping::new(
        "id000000000000000001".into(),
        "t1".into(),
        Some("v1".into()),
        Some("127.0.0.1:80".parse().unwrap()),
        Some(1),
    );
    let body: KrpcBody = Query::Ping(ping).into();
    let req = Request::new(body, remote_addr, local_addr);
    let cnt = client.send(req).await.unwrap();
    println!("send {cnt} bytes");

    let ping_reply = PingOrAnnounceReply::new(
        "id000000000000000001".into(),
        "t1".into(),
        Some("v1".into()),
        Some("127.0.0.1:80".parse().unwrap()),
        Some(1),
    );
    let body: KrpcBody = Reply::PingOrAnnounce(ping_reply).into();
    let req = Request::new(body, remote_addr, local_addr);

    let cnt = client.send(req).await.unwrap();
    println!("send {cnt} bytes");
}

fn build_socket(socket_addr: SocketAddr) -> Result<UdpSocket, Error> {
    let std_sock = std::net::UdpSocket::bind(socket_addr)
        .map_err(|e| Error::new_bind(Some(Box::new(e))))?;
    std_sock
        .set_nonblocking(true)
        .map_err(|e| Error::new_bind(Some(Box::new(e))))?;

    let socket =
        UdpSocket::from_std(std_sock).map_err(|e| Error::new_bind(Some(Box::new(e))))?;

    Ok(socket)
}