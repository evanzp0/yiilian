use std::{collections::HashMap, sync::Arc};

use tokio::sync::mpsc;
use yiilian_core::common::shutdown::{spawn_with_shutdown, ShutdownReceiver};

use crate::common::context::Context;

use super::{EventListener, EventCommand, Event};

/// 事件管理
#[derive(Debug)]
pub struct EventManager {
    event_tx: mpsc::Sender<EventCommand>,
}

impl EventManager {
    // 生成 EventManager，并开启任务监听循环
    pub fn new(shutdown: ShutdownReceiver) -> EventManager {
        let (event_tx, event_rx) = mpsc::channel::<EventCommand>(1);

        let evt_mgr = EventManager {
            event_tx,
        };

        // 事件循环
        run_event_loop(shutdown, event_rx);

        evt_mgr
    }

    /// 添加事件监听器
    pub async fn add_listener(&self, listener: Arc<dyn EventListener>) {
        let cmd = EventCommand::AddEventListener(listener);
        
        if let Err(e) = self.event_tx.send(cmd).await {
            log::error!(target: "yiilian_core::event::add_listener", "add listener failed: {}", e);
        }
    }

    /// 发送事件
    pub async fn emit(&self, event: Event) {
        let cmd = EventCommand::EmitEvent(event);      
        if let Err(e) = self.event_tx.send(cmd).await {
            log::error!(target: "yiilian_core::event::emit", "emit failed: {}", e);
        }
    }
}

/// 事件循环
fn run_event_loop(shutdown: ShutdownReceiver, mut event_rx: mpsc::Receiver<EventCommand>) {
    spawn_with_shutdown(
        shutdown.clone(),
        async move {
            let mut listener_map: HashMap<String, Vec<Arc<dyn EventListener>>> = HashMap::new();
            
            loop {
                let cmd = event_rx.recv().await;
                match cmd {
                    Some(cmd) => match cmd {
                        EventCommand::AddEventListener(listener) => {
                            let listeners = if let Some(listeners) = listener_map.get_mut(listener.get_event_type()) {
                                listeners
                            } else {
                                listener_map.insert(listener.get_event_type().to_owned(), vec![]);
                                listener_map.get_mut(listener.get_event_type()).unwrap()
                            };
    
                            listeners.push(listener);
                        },
                        EventCommand::EmitEvent(event) => {
                            // log::trace!("EmmitEvent: {:?}", event);
                            if let Some(listeners) = listener_map.get(event.get_type()) {
                                for listener in listeners {
                                    let listener = (*listener).clone();
                                    let event = event.clone();
                                    let shutdown = shutdown.clone();

                                    // 使用新任务是为了防止 listener.apply() 时，任务内部发送不可捕获的异常导致消息通道被破坏
                                    tokio::spawn(async move {
                                        tokio::select! {
                                            _ = async {
                                                listener.apply(event)
                                            } => (),
                                            _= shutdown.watch() => (),
                                        }
                                    });
                                }
                            }
                        },
                    },
                    None => break,
                }
            }
        },
        "main event loop",
        None
    );
}

