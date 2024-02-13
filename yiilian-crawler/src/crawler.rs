use std::net::SocketAddr;

use bytes::Bytes;
use rand::thread_rng;
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
};
use yiilian_core::common::error::Error;
use yiilian_dht::common::Id;

use crate::data::frame::{Handshake, MESSAGE_EXTENSION_ENABLE};

pub struct Crawler;

impl Crawler {
    pub async fn download_metadata(
        info_hash: &[u8],
        peer_address: SocketAddr,
    ) -> Result<(), Error> {
        let mut stream = TcpStream::connect(peer_address)
            .await
            .map_err(|error| Error::new_net(Some(error.into()), None, Some(peer_address)))?;

        let peer_id = Id::from_random(&mut thread_rng()).get_bytes();
        let hs = Handshake::new(&MESSAGE_EXTENSION_ENABLE, info_hash, &peer_id);
        let hs: Bytes = hs.into();
        stream
            .write_all(&hs)
            .await
            .map_err(|error| Error::new_net(Some(error.into()), None, Some(peer_address)))?;



        // let socket = TcpSocket::new_v4()
        //     .map_err(|error| Error::new_net(Some(error.into()), None, Some(peer_address)))?;
        // let mut stream = socket.connect(peer_address)
        //     .await
        //     .map_err(|error| Error::new_net(Some(error.into()), None, Some(peer_address)))?;

        // let mut buf = vec![];
        // stream.read(&mut buf).await.unwrap();

        todo!()
    }
}
