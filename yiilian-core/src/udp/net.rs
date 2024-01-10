use std::{net::SocketAddr, sync::Arc};

use bytes::Bytes;
use tokio::net::UdpSocket;

use crate::common::error::Error;

type Result<T> = std::result::Result<T, Error>;

/// 通过 socket 接收一个帧，并返回该帧和对方地址
pub async fn recv_from(socket: &Arc<UdpSocket>) -> Result<(Bytes, SocketAddr)> {
    let mut buf = [0; 65000];
    let (len, remote_addr) = socket
        .recv_from(&mut buf)
        .await
        .map_err(|e| Error::new_io(Some(e.into()), None))?;

    let data = Bytes::from(buf[..len].to_owned());
    Ok((data, remote_addr))
}

/// 通过 socket 发送一个帧
pub async fn send_to(socket: Arc<UdpSocket>, data: &Bytes, dest: SocketAddr) -> Result<()> {
    // log::trace!(target:"yiilian_core::udp", "Sending {} bytes to {}", bytes.len(), dest);

    match socket.send_to(data, dest).await {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::new_io(Some(e.into()), Some(dest))),
    }
}