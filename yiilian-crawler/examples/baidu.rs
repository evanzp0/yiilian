use std::{error::Error, net::SocketAddr};
use tokio::{io::AsyncWriteExt, net::TcpStream, time::timeout};
use yiilian_crawler::net::tcp::read_all;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    let peer_address: SocketAddr = "180.101.50.188:80".parse().unwrap();

    let content = "GET / HTTP/1.0\r\nHost:www.baidu.com\r\n\r\n";

    let net_timeout = tokio::time::Duration::from_millis(5000);

    let mut stream = timeout(net_timeout, TcpStream::connect(peer_address)).await??;
    
    timeout(net_timeout, stream.write_all(&content.as_bytes())).await??;

    let rst = timeout(net_timeout, read_all(&mut stream)).await??;

    println!("{}", String::from_utf8_lossy(&rst));

    Ok(())
}
