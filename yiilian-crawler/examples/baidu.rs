use std::net::SocketAddr;
use tokio::{io::AsyncWriteExt, net::TcpStream};
use yiilian_core::common::error::Error;
use yiilian_crawler::net::tcp::read_all;

#[tokio::main]
async fn main() {
    let peer_address: SocketAddr = "180.101.50.188:80".parse().unwrap();

    let content = "GET / HTTP/1.0\r\nHost:www.baidu.com\r\n\r\n";

    let mut stream = TcpStream::connect(peer_address)
        .await
        .map_err(|error| Error::new_net(Some(error.into()), None, Some(peer_address)))
        .unwrap();

    stream
        .write_all(&content.as_bytes())
        .await
        .map_err(|error| Error::new_net(Some(error.into()), None, Some(peer_address)))
        .unwrap();

    let rst = read_all(&mut stream).await.unwrap();

    println!("{}", String::from_utf8_lossy(&rst));
}
