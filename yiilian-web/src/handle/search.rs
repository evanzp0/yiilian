use std::collections::HashMap;

use axum::{extract::Query, response::Html};
use tantivy::{collector::TopDocs, query::QueryParser, ReloadPolicy};
use tracing::{instrument, trace};

use crate::{common::app_state, render};

#[instrument]
pub async fn search(Query(params): Query<HashMap<String, String>>) -> Html<String> {
    if let Some(q) = params.get("q") {
        let reader = app_state()
            .index()
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()
            .unwrap();

        let searcher = reader.searcher();

        let schema = app_state().index().schema();
        let file_paths = schema.get_field("file_paths").unwrap();

        let query_parser = QueryParser::for_index(app_state().index(), vec![file_paths]);
        let query = query_parser.parse_query(q).unwrap();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();

        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address).unwrap();
            trace!("{:#?}", schema.to_json(&retrieved_doc));
        }

        render!("index.tpl", q).into()
    } else {
        render!("index.tpl").into()
    }
}