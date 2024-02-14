use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use bytes::{Bytes, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use yiilian_core::{common::error::Error, net::tcp::read};

use crate::data::frame::{Handshake, HANDSHAKE_LEN, MESSAGE_EXTENSION_ENABLE, MESSAGE_LEN_PREFIX};

pub async fn send_handshake(
    stream: &mut TcpStream,
    info_hash: &[u8],
    peer_id: &[u8],
) -> Result<(), Error> {
    let hs = Handshake::new(&MESSAGE_EXTENSION_ENABLE, &info_hash, &peer_id);
    let hs: Bytes = hs.into();

    stream
        .write_all(&hs)
        .await
        .map_err(|error| Error::new_net(Some(error.into()), None, None))?;

    Ok(())
}

pub async fn read_handshake(stream: &mut TcpStream) -> Result<Bytes, Error> {
    let mut buf: [u8; HANDSHAKE_LEN] = [0; HANDSHAKE_LEN];
    read(stream, &mut buf)
        .await
        .map_err(|error| Error::new_net(Some(error.into()), None, None))?;

    let rst: Bytes = buf[..].to_owned().into();

    Ok(rst)
}

pub async fn read_message(stream: &mut TcpStream) -> Result<Bytes, Error> {
    let mut buf: [u8; MESSAGE_LEN_PREFIX] = [0; MESSAGE_LEN_PREFIX];
    read(stream, &mut buf)
        .await
        .map_err(|error| Error::new_net(Some(error.into()), None, None))?;
    let message_len_bytes = &buf[..];
    let message_len =
        u32::from_be_bytes(message_len_bytes.try_into().expect("bytes len is invalid")) as usize
            + MESSAGE_LEN_PREFIX;

    let buf: Vec<u8> = vec![0; message_len as usize];
    let mut buf = Cursor::new(buf);
    buf.seek(SeekFrom::Start(0)).unwrap();
    
    std::io::Write::write(&mut buf, message_len_bytes).unwrap();
    let mut buf: Vec<u8> = buf.into_inner();
    
    read(stream, &mut buf[MESSAGE_LEN_PREFIX..])
        .await
        .map_err(|error| Error::new_net(Some(error.into()), None, None))?;

    Ok(buf.into())
}

pub async fn send_message(stream: &mut TcpStream, data: &[u8]) -> Result<(), Error> {
    stream
        .write_all(data)
        .await
        .map_err(|error| Error::new_net(Some(error.into()), None, None))?;

    Ok(())
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
            Err(e) => Err(Error::new_net(Some(e.into()), None, None))?,
        }
    }

    Ok(rst.into())
}
