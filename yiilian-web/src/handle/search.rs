use std::collections::HashMap;

use anyhow::anyhow;
use axum::{extract::Query, response::Html};
use tantivy::{collector::TopDocs, query::QueryParser, ReloadPolicy};
use tracing::{instrument, trace};

use crate::{
    common::{app_state, WebError},
    render, Result,
};

#[instrument]
pub async fn search(Query(params): Query<HashMap<String, String>>) -> Result<Html<String>> {
    if let Some(q) = params.get("q") {
        let reader = app_state()
            .index()
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;

        let searcher = reader.searcher();

        let schema = app_state().index().schema();

        let info_hash = schema
            .get_field("info_hash")
            .ok_or(WebError::from_error(anyhow!(
                "Field 'info_hash' not found in schema"
            )))?;
        let file_paths = schema
            .get_field("file_paths")
            .ok_or(WebError::from_error(anyhow!(
                "Field 'file_paths' not found in schema"
            )))?;

        let query_parser = QueryParser::for_index(app_state().index(), vec![info_hash, file_paths]);
        let query = query_parser.parse_query(q)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;

        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;
            trace!("{:#?}", schema.to_json(&retrieved_doc));
        }

        Ok(render!("index.tera", q)?.into())
    } else {
        Ok(render!("index.tera")?.into())
    }
}
