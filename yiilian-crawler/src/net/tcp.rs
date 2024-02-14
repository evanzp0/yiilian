
use bytes::{Bytes, BytesMut};
use tokio::{io::AsyncReadExt, net::TcpStream};
use yiilian_core::common::error::Error;

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