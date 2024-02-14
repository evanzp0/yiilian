use bytes::{Bytes, BytesMut};
use tokio::{io::AsyncReadExt, net::TcpStream};
use yiilian_core::{common::error::Error, net::tcp::read};

use crate::data::frame::{HANDSHAKE_LEN, MESSAGE_LEN_PREFIX};

pub async fn read_handshake(stream: &mut TcpStream) -> Result<Bytes, Error> {
    let mut buf:[u8; HANDSHAKE_LEN] = [0; HANDSHAKE_LEN];
    read(stream, &mut buf)
        .await
        .map_err(|error| Error::new_net(Some(error.into()), None, None))?;

    let rst: Bytes = buf[..].to_owned().into();

    Ok(rst)
}

pub async fn read_message(stream: &mut TcpStream) -> Result<Bytes, Error> {
    let mut buf:[u8; MESSAGE_LEN_PREFIX] = [0; MESSAGE_LEN_PREFIX];
    read(stream, &mut buf)
        .await
        .map_err(|error| Error::new_net(Some(error.into()), None, None))?;

    let message_len = u32::from_be_bytes(buf[..].try_into().expect("bytes len is invalid"));

    let mut buf: Vec<u8> = vec![0; message_len as usize];
    let val = read(stream, &mut buf)
        .await
        .map_err(|error| Error::new_net(Some(error.into()), None, None))?;

    Ok(buf.into())
}

pub async fn read_all(stream: &mut TcpStream) -> Result<Bytes, Error> {
    let mut rst = BytesMut::new();
    loop {
        let mut buf = [0; 4096];

        match stream.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                // println!("{n}");
                println!("{:?}", &buf[0..n]);
                rst.extend(&buf[0..n]);
            }
            Err(e) => {
                Err(Error::new_net(Some(e.into()), None, None))?
            }
        }
    }

    Ok(rst.into())
}