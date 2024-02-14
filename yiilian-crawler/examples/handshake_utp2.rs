
use std::net::SocketAddr;
use utp_rs::conn::ConnectionConfig;
use utp_rs::socket::UtpSocket;

#[tokio::main]
async fn main() {
	// bind a standard UDP socket. (transport is over a `tokio::net::UdpSocket`.)
	let socket_addr = SocketAddr::from(([0, 0, 0, 0], 16500));
	let udp_socket = UtpSocket::bind(socket_addr).await.unwrap();

	// connect to a remote peer over uTP.
	let peer_address: SocketAddr = "142.215.164.105:6882".parse().unwrap();
	let config = ConnectionConfig::default();
	let _stream = udp_socket.connect(peer_address, config).await.unwrap();

    // let info_hash = "fad4c3aa007a90b2c0635fc39e8e5b16da8c410d";
    // let info_hash = hex::decode(info_hash)
    //     .map_err(|hex_err| Error::new_id(Some(hex_err.into()), None))
    //     .unwrap();
    // let peer_id = Id::from_random(&mut thread_rng()).get_bytes();
    // let hs = Handshake::new(&MESSAGE_EXTENSION_ENABLE, &info_hash, &peer_id);
    // let hs: Bytes = hs.into();

    println!("connected");

    // stream.write(&hs).await.unwrap();

    // let rst = read_all(&mut stream).unwrap();
    // println!("{:?}", rst);
}
