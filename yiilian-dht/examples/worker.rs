use std::{sync::Arc, time::Duration};

use tokio::{sync::Semaphore, time::sleep};

#[tokio::main]
async fn main() {
    let workers = Arc::new(Semaphore::new(10));

    loop {
        println!("1: {}", workers.available_permits());
        let worker = workers.clone().acquire_owned().await.unwrap();
        println!("2: {}", workers.available_permits());

        let workers_i = workers.clone();
        tokio::spawn(async move {
            sleep(Duration::from_secs(1)).await;
            drop(worker);
            println!("3: {}", workers_i.available_permits());
        });
    }
}