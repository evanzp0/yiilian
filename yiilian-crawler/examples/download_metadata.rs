use std::net::SocketAddr;

use rand::thread_rng;
use yiilian_core::common::error::Error;

use yiilian_crawler::crawler::Crawler;
use yiilian_dht::common::Id;

#[tokio::main]
async fn main() {
    let peer_address: SocketAddr = "192.168.31.6:15000".parse().unwrap();
    let info_hash = "FA84A39C18D5960B0272D3E1D2A7900FB09F5EB3";
    let info_hash = hex::decode(info_hash)
        .map_err(|hex_err| Error::new_id(Some(hex_err.into()), None))
        .unwrap();

    let peer_id = Id::from_random(&mut thread_rng()).get_bytes();

    println!("connected");

    let crawler = Crawler::new();
    let metadata = crawler.fetch_metdata(peer_address, &info_hash, &peer_id).await.unwrap();

    println!("{:?}", metadata);
} 