use std::error::Error;

use tokio::{io::AsyncReadExt, net::TcpStream, time::timeout};

// read reads size-length bytes from conn to data.
pub async fn read(stream: &mut TcpStream, buf: &mut [u8]) -> Result<usize, Box<dyn Error + Send + Sync>> {
    let duration = tokio::time::Duration::from_secs(15);
    
    let n = timeout(duration, stream.read_exact(buf)).await??;

    Ok(n)
}