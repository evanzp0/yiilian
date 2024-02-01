use std::{future::Future, time::Duration};

use log::{error, trace, warn};
use tokio::{sync::{mpsc, watch}, time::sleep};

/// 包含了用于在异步任务中，等待 "关闭信号" 的方法
#[derive(Clone, Debug)]
pub struct ShutdownReceiver {
    /// 关闭信号接收端
    shutdown_rx: watch::Receiver<bool>,
    /// 确认关闭发送端
    /// ShutdownReceiver drop 时，该通道接收端自动关闭。
    _shutdown_confirm_tx: mpsc::Sender<bool>,
}

impl ShutdownReceiver {
    /// 等待 ShutdownSender 发出的关闭信号
    ///
    /// ShutdownReceiver 必须在该方法返回后被 drop 。
    pub async fn watch(mut self) {
        if let Err(e) = self.shutdown_rx.changed().await {
            error!("Error watching shutdown_rx : {:?}", e);
        }
    }
}

/// 包含了将"关闭信号"，发送给异步任务的方法
pub struct ShutdownSender {
    /// 关闭信号发送端
    shutdown_tx: watch::Sender<bool>,
    /// 确认关闭接收端
    shutdown_confirm_rx: mpsc::Receiver<bool>,
}

impl ShutdownSender {
    /// 发送关闭信号给所有在等待 [ShutdownReceiver](crate::shutdown::ShutdownReceiver) 的异步任务，让它们停止工作。
    ///
    /// 等待那些异步任务全部关闭（ShutdownReceivers 全部被 drop）
    pub async fn shutdown(&mut self) {
        // 发送关闭信号
        if let Err(e) = self.shutdown_tx.send(true) {
            warn!("Failed to send shutdown signal: {:?}", e);
        }

        // 等待所有异步任务的确认关闭
        let _ = self.shutdown_confirm_rx.recv().await;
    }
}

/// 用于生成新的可优雅关闭的异步任务
/// 
/// future: 要执行的异步任务
/// task_name: 异步任务名
/// timeout: 为异步任务执行设置的超时
pub fn spawn_with_shutdown<T>(
    shutdown: ShutdownReceiver,
    future: T,
    task_name: impl std::fmt::Display + Send + 'static + Sync,
    timeout: Option<Duration>,
) where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    tokio::spawn(async move {
        trace!("Task '{}' starting up", task_name);
        tokio::select! {
            _ = shutdown.watch() => (),
            _ = future => (),
            _ = async {
                match timeout {
                    Some(timeout) => {
                        sleep(timeout).await;
                        trace!("Task '{}' timed out", task_name);
                    },
                    None => { std::future::pending::<bool>().await; },
                }
            } => (),
        }
    });
}

/// 创建一对关联的 ShutdownSender 和 ShutdownReceiver 。
/// 其中 ShutdownReceiver 的 [watch](crate::shutdown::ShutdownReceiver::watch) 方法会一直等待，
/// 直到 ShutdownSender 的 [shutdown](crate::shutdown::ShutdownSender::shutdown) 被调用。
///
/// 在异步任务中应当使用 ShutdownReceiver 克隆体
pub fn create_shutdown() -> (ShutdownSender, ShutdownReceiver) {
    // 使用该 channel 发送关闭信号给所有异步任务
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // 使用该通道来确认所有异步任务都关闭了
    let (shutdown_confirm_tx, shutdown_confirm_rx) = mpsc::channel::<bool>(1);

    (
        ShutdownSender {
            shutdown_tx,
            shutdown_confirm_rx,
        },
        ShutdownReceiver {
            shutdown_rx,
            _shutdown_confirm_tx: shutdown_confirm_tx,
        },
    )
}


#[cfg(test)]
mod tests {

    use super::*;

    async fn run() {
        loop {
            sleep(Duration::from_secs(1)).await;
        }
    }

    #[tokio::test]
    async fn test() {
        let (mut shutdown_tx, shutdown_rx) = create_shutdown();

        let task = async {
            spawn_with_shutdown(
                shutdown_rx, 
                async {
                    println!("hello!!!!!!!");
                },
                "shutdown_task",
                Some(Duration::from_secs(10))
            );
        };

        tokio::select! {
            _ = run() => (),
            _ = task => (),
            _ = tokio::signal::ctrl_c() => {
                shutdown_tx.shutdown().await;
            },
        }
    }
}