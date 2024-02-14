use std::io::Write;
use std::net::SocketAddr;

use bytes::Bytes;
use rand::thread_rng;
use utp::UtpStream;
use yiilian_core::common::error::Error;
use yiilian_crawler::data::frame::{Handshake, MESSAGE_EXTENSION_ENABLE};
use yiilian_crawler::net::utp::read_all;
use yiilian_dht::common::Id;

#[tokio::main]
async fn main() {
    let peer_address: SocketAddr = "192.168.31.6:15000".parse().unwrap();
    let info_hash = "FA84A39C18D5960B0272D3E1D2A7900FB09F5EB3";
    let info_hash = hex::decode(info_hash)
        .map_err(|hex_err| Error::new_id(Some(hex_err.into()), None))
        .unwrap();

    let peer_id = Id::from_random(&mut thread_rng()).get_bytes();
    let hs = Handshake::new(&MESSAGE_EXTENSION_ENABLE, &info_hash, &peer_id);
    let hs: Bytes = hs.into();

    let mut stream = UtpStream::connect(peer_address)
        .unwrap();

    println!("connected");

    stream
        .write(&hs)
        .map_err(|error| Error::new_net(Some(error.into()), None, Some(peer_address)))
        .unwrap();
    stream.flush().unwrap();

    println!("write handshake");

    let rst = read_all(&mut stream).unwrap();

    println!("read handshake");

    println!("{:?}", rst);
}
