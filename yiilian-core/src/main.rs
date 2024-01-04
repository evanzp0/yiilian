use std::{time::Duration, future::Future, task::{Context, Poll}};

use pin_project::pin_project;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() {
    sleep_me().await.ok();
}

fn sleep_me() -> JoinHandle<()> {
    let sp = tokio::time::sleep(Duration::from_secs(5));
    let sp = SleepMe::new(sp);

    tokio::spawn(async move {
        sp.await
    })
}

#[pin_project]
struct SleepMe<F> {
    #[pin]
    future: F
}

impl<F> SleepMe<F> {
    pub fn new(future: F) -> Self {
        Self { future }
    }
}

impl<F> Future for SleepMe<F>
where
    F: Future<Output = ()>,
{
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.project();
        match me.future.poll(cx) {
            Poll::Pending => {
                println!("i am not ready");
                return Poll::Pending
            },
            Poll::Ready(_) => {
                println!("i am ready");
                return Poll::Ready(())
            },
        }
    }
}