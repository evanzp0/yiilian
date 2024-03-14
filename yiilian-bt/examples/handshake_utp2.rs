
use std::net::SocketAddr;
use bytes::Bytes;
use rand::thread_rng;
use utp_rs::conn::ConnectionConfig;
use utp_rs::socket::UtpSocket;
use yiilian_bt::data::frame::{Handshake, MESSAGE_EXTENSION_ENABLE};
use yiilian_dht::common::Id;

#[tokio::main]
async fn main() {
	// bind a standard UDP socket. (transport is over a `tokio::net::UdpSocket`.)
	let socket_addr: SocketAddr = "0.0.0.0:16500".parse().unwrap();
	let udp_socket = UtpSocket::bind(socket_addr).await.unwrap();

	// connect to a remote peer over uTP.
	let peer_address: SocketAddr = "192.168.31.6:15000".parse().unwrap();
	let config = ConnectionConfig::default();
	let mut stream = udp_socket.connect(peer_address, config).await.unwrap();

    println!("connected");

    let info_hash = "FA84A39C18D5960B0272D3E1D2A7900FB09F5EB3";
    let info_hash = hex::decode(info_hash)
        .unwrap();
    let peer_id = Id::from_random(&mut thread_rng()).get_bytes();
    let hs = Handshake::new(&MESSAGE_EXTENSION_ENABLE, &info_hash, &peer_id);
    let hs: Bytes = hs.into();

    stream.write(&hs).await.unwrap();
    println!("write handshake");

    let mut data = vec![];
	let _n = stream.read_to_eof(&mut data).await.unwrap();

    println!("{:?}", data);
}
