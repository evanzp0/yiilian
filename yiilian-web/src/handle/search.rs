use std::collections::HashMap;

use anyhow::anyhow;
use axum::{extract::Query, response::Html};
use tantivy::{collector::TopDocs, query::QueryParser, ReloadPolicy};
use tracing::{instrument, trace};
use yiilian_index::res_info_doc::ResInfoDoc;

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
        let res_type = schema
            .get_field("res_type")
            .ok_or(WebError::from_error(anyhow!(
                "Field 'res_type' not found in schema"
            )))?;
        let create_time = schema
            .get_field("create_time")
            .ok_or(WebError::from_error(anyhow!(
                "Field 'create_time' not found in schema"
            )))?;
        let file_paths = schema
            .get_field("file_paths")
            .ok_or(WebError::from_error(anyhow!(
                "Field 'file_paths' not found in schema"
            )))?;
        let file_sizes = schema
            .get_field("file_sizes")
            .ok_or(WebError::from_error(anyhow!(
                "Field 'file_sizes' not found in schema"
            )))?;

        let query_parser = QueryParser::for_index(app_state().index(), vec![info_hash, file_paths]);
        let query = query_parser.parse_query(q)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;

        let mut rst_docs = vec![];
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;

            let info_hash = retrieved_doc.get_first(info_hash).unwrap().as_text().unwrap().to_owned();
            let res_type = retrieved_doc.get_first(res_type).unwrap().as_u64().unwrap() as i32;
            let create_time = retrieved_doc.get_first(create_time).unwrap().as_text().unwrap().to_owned();

            let mut file_path_list = vec![];
            for file_path in retrieved_doc.get_all(file_paths) {
                file_path_list.push(file_path.as_text().unwrap().to_owned());
            }

            let mut file_size_list = vec![];
            for file_size in retrieved_doc.get_all(file_sizes) {
                file_size_list.push(file_size.as_u64().unwrap() as i64);
            }

            let info_doc = ResInfoDoc { 
                info_hash, 
                res_type, 
                create_time, 
                file_paths: file_path_list, 
                file_sizes: file_size_list,
            };
            trace!("{:#?}", info_doc);

            rst_docs.push(info_doc)
        }

        Ok(
            render!(
                "index.tera", 
                {
                    "q" => q,
                    "info_docs" => rst_docs,
                }
            )?
            .into()
        )
    } else {
        Ok(render!("index.tera")?.into())
    }
}
