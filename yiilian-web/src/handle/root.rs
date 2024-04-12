use crate::render;

pub async fn root() -> String {
    tracing::trace!("index");
    
    render!("index.tpl", { "name" => "hello world", "val" => &2 })
}