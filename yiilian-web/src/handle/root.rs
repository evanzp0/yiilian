use axum::response::Html;
use tracing::instrument;

use crate::render;

#[instrument]
pub async fn root() -> Html<String> {
    
    render!("index.tpl").into()
}