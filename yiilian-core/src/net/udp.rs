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
pub async fn send_to(socket: &Arc<UdpSocket>, data: &Bytes, dest: SocketAddr) -> Result<usize> {
    match socket.send_to(data, dest).await {
        Ok(val) => Ok(val),
        Err(e) => {
            #[cfg(target_os = "linux")]
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                return Err(Error::new_conntrack(
                    Some(e.into()),
                    Some(
                        "send_to resulted in PermissionDenied. Is conntrack table full?".to_owned(),
                    ),
                    Some(dest),
                ));
            }

            Err(Error::new_io(Some(e.into()), Some(dest)))
        }
    }
}
