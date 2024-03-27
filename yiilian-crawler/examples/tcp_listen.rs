use std::io;

use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:18080").await?;

    println!("Listening at: {}", listener.local_addr().unwrap());

    while let Ok((_stream, remote_addr)) = listener.accept().await {
        println!("new client: {:?}", remote_addr);
    }

    Ok(())
}