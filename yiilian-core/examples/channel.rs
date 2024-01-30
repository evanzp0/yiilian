use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    let (tx, mut rx1) = broadcast::channel(1);
    
    let _rst = tx.send(10);
    let rst = tx.send(20);
    println!("{:?}", rst);

    let rst = rx1.recv().await;
    println!("{:?}", rst);

    let rst = rx1.recv().await;
    println!("{:?}", rst);

    // drop(tx);
    // let rst = rx1.recv().await;
    // println!("{:?}", rst);

    drop(rx1);
    let rst = tx.send(30);
    println!("{:?}", rst);

    let mut rx2 = tx.subscribe();
    let rst = tx.send(40);
    println!("{:?}", rst);

    let rst = rx2.recv().await;
    println!("{:?}", rst);
}