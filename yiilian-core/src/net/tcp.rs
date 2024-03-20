use std::error::Error as StdError;

use bytes::Bytes;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream, time::timeout};

use crate::{common::error::Error, data::{BtHandshake, HANDSHAKE_LEN, MESSAGE_EXTENSION_ENABLE}};

// read reads size-length bytes from conn to data.
pub async fn read(stream: &mut TcpStream, buf: &mut [u8]) -> Result<usize, Box<dyn StdError + Send + Sync>> {
    let duration = tokio::time::Duration::from_secs(10);
    
    let n = timeout(duration, stream.read_exact(buf)).await??;
    // println!("{n}, {:?}", buf);

    Ok(n)
}

pub async fn send_bt_handshake(
    stream: &mut TcpStream,
    info_hash: &[u8],
    peer_id: &[u8],
) -> Result<(), Error> {
    let hs = BtHandshake::new(&MESSAGE_EXTENSION_ENABLE, &info_hash, &peer_id);
    let hs: Bytes = hs.into();

    stream
        .write_all(&hs)
        .await
        .map_err(|error| Error::new_net(Some(error.into()), Some("send_handshake".to_owned()), None))?;

    Ok(())
}

pub async fn read_bt_handshake(stream: &mut TcpStream) -> Result<Bytes, Error> {
    let mut buf: [u8; HANDSHAKE_LEN] = [0; HANDSHAKE_LEN];
    read(stream, &mut buf)
        .await
        .map_err(|error| Error::new_net(Some(error.into()), None, None))?;

    let rst: Bytes = buf[..].to_owned().into();

    Ok(rst)
}
