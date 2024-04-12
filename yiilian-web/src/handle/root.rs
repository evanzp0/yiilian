use axum::response::Html;
use tracing::instrument;

use crate::{render, Result};

#[instrument]
pub async fn root() -> Result<Html<String>> {
    
    Ok(render!("index.tera")?.into())
}