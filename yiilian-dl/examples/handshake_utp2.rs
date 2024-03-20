
use std::net::SocketAddr;
use bytes::Bytes;
use rand::thread_rng;
use utp_rs::conn::ConnectionConfig;
use utp_rs::socket::UtpSocket;
use yiilian_core::data::{BtHandshake, MESSAGE_EXTENSION_ENABLE};
use yiilian_dht::common::Id;

#[tokio::main]
async fn main() {
	// bind a standard UDP socket. (transport is over a `tokio::net::UdpSocket`.)
	let socket_addr: SocketAddr = "0.0.0.0:16500".parse().unwrap();
	let udp_socket = UtpSocket::bind(socket_addr).await.unwrap();

	// connect to a remote peer over uTP.
	let peer_address: SocketAddr = "46.49.81.151:7872".parse().unwrap();
	let config = ConnectionConfig::default();
	let mut stream = udp_socket.connect(peer_address, config).await.unwrap();

    println!("connected");

    let info_hash = "CA898835A835E4CF4C995CBC09F5AC47A1BF69D3";
    let info_hash = hex::decode(info_hash)
        .unwrap();
    let peer_id = Id::from_random(&mut thread_rng()).get_bytes();
    let hs = BtHandshake::new(&MESSAGE_EXTENSION_ENABLE, &info_hash, &peer_id);
    let hs: Bytes = hs.into();

    stream.write(&hs).await.unwrap();
    println!("write handshake");

    let mut data = vec![];
	let _n = stream.read_to_eof(&mut data).await.unwrap();

    println!("{:?}", data);
}
