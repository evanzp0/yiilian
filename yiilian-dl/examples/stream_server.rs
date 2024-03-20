use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}};


#[tokio::main]
async fn main() {
    let mut data = [0u8; 12];
    let listener = TcpListener::bind("127.0.0.1:34250").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        let mut stream: TcpStream = TcpStream::connect(addr).await.unwrap();
        stream.write_all(b"Hello ").await.unwrap();
        stream.flush().await.unwrap();

        // drop(stream);

        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // let mut stream: TcpStream = TcpStream::connect(addr).await.unwrap();
        
        stream.write_all(b"world!!").await.unwrap();
    });

    let (mut tokio_tcp_stream, _) = listener.accept().await.unwrap();
    // let mut std_tcp_stream = tokio_tcp_stream.into_std().unwrap();
    handle.await.expect("The task being joined has panicked");
    // std_tcp_stream.set_nonblocking(false).unwrap();
    // std_tcp_stream.read_exact(&mut data).unwrap();
    tokio_tcp_stream.read_exact(&mut data).await.unwrap();

    assert_eq!(b"Hello world!", &data);
}