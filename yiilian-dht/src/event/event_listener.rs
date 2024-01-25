use std::sync::Arc;

use async_trait::async_trait;

use super::Event;

#[async_trait]
/// 事件监听器
pub trait EventListener: Send + Sync + 'static + core::fmt::Debug {
    async fn apply(&self, event: Event);
    fn get_event_type(&self) -> &str;
}

pub enum EventCommand {
    AddEventListener(Arc<dyn EventListener>),
    EmitEvent(Event),
}