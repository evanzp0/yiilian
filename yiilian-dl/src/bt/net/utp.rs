use std::io::Read;

use bytes::{Bytes, BytesMut};
use utp::UtpStream;
use yiilian_core::common::error::Error;


pub fn read_all(stream: &mut UtpStream) -> Result<Bytes, Error> {
    let mut rst = BytesMut::new();
    loop {
        let mut buf = [0; 4096];

        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                rst.extend(&buf[0..n]);
            }
            Err(e) => {
                Err(Error::new_net(Some(e.into()), None, None))?
            }
        }
    }

    Ok(rst.into())
}