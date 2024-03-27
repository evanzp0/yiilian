use std::net::SocketAddr;

use tokio::net::UdpSocket;

#[tokio::main]
async fn main() {
    let socket_addr: SocketAddr = "0.0.0.0:34636".parse().unwrap();

    let std_sock =
        std::net::UdpSocket::bind(socket_addr).unwrap();
    std_sock
        .set_nonblocking(true)
        .unwrap();

    let socket = UdpSocket::from_std(std_sock).unwrap();

    println!("{}", socket_addr);

    println!("{}", socket.local_addr().unwrap());
}
