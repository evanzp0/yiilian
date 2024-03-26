use std::io;

use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:18080").await?;

    println!("Listening at: {}", listener.local_addr().unwrap());

    match listener.accept().await {
        Ok((_socket, addr)) => println!("new client: {:?}", addr),
        Err(e) => println!("couldn't get client: {:?}", e),
    }

    Ok(())
}