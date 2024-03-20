use std::io::{Cursor, Seek, SeekFrom};

use bytes::{Bytes, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use yiilian_core::{common::error::Error, net::tcp::read};

use crate::bt::data::frame::MESSAGE_LEN_PREFIX;

pub async fn read_message(stream: &mut TcpStream) -> Result<Bytes, Error> {
    let mut buf: [u8; MESSAGE_LEN_PREFIX] = [0; MESSAGE_LEN_PREFIX];
    read(stream, &mut buf)
        .await
        .map_err(|error| Error::new_net(Some(error.into()), Some("read_message [u8; MESSAGE_LEN_PREFIX]".to_owned()), None))?;
    let message_len_bytes = &buf[..];
    let message_len =
        u32::from_be_bytes(message_len_bytes.try_into().expect("bytes len is invalid")) as usize
            + MESSAGE_LEN_PREFIX;

    let buf: Vec<u8> = vec![0; message_len as usize];
    let mut buf = Cursor::new(buf);
    buf.seek(SeekFrom::Start(0))
        .map_err(|error| Error::new_net(Some(error.into()), Some("seek message error".to_owned()), None))?;
    
    std::io::Write::write(&mut buf, message_len_bytes)
        .map_err(|error| Error::new_net(Some(error.into()), Some("write message error".to_owned()), None))?;
    let mut buf: Vec<u8> = buf.into_inner();
    
    read(stream, &mut buf[MESSAGE_LEN_PREFIX..])
        .await
        .map_err(|error| Error::new_net(Some(error.into()), Some("read_message [MESSAGE_LEN_PREFIX..]".to_owned()), None))?;

    Ok(buf.into())
}

pub async fn send_message(stream: &mut TcpStream, data: &[u8]) -> Result<(), Error> {
    stream
        .write_all(data)
        .await
        .map_err(|error| Error::new_net(Some(error.into()), Some("send_message".to_owned()), None))?;

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
            Err(e) => Err(Error::new_net(Some(e.into()), Some("read_all".to_owned()), None))?,
        }
    }

    Ok(rst.into())
}
