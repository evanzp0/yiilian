use tantivy::{
    collector::TopDocs, query::QueryParser, schema::{Schema, STORED, TEXT}, Index, ReloadPolicy
};
use tempfile::TempDir;
use yiilian_index::res_info_doc::ResInfoDoc;

fn main() -> tantivy::Result<()> {
    let index_path = TempDir::new()?;

    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("info_hash", TEXT | STORED);
    schema_builder.add_i64_field("res_type", STORED);
    schema_builder.add_text_field("create_time", STORED);
    schema_builder.add_text_field("file_paths", TEXT | STORED);
    schema_builder.add_i64_field("file_sizes", STORED);

    let schema = schema_builder.build();

    let index = Index::create_in_dir(&index_path, schema.clone())?;
    let mut index_writer = index.writer(50_000_000)?;

    // let info_hash = schema.get_field("info_hash").unwrap();
    let file_paths = schema.get_field("file_paths").unwrap();

    let res_doc = ResInfoDoc {
        info_hash: "00000000000000000001".to_owned(),
        res_type: 0,
        create_time: "2024-11-10T11:00:00".to_owned(),
        file_paths: vec!["file1".to_owned()],
        file_sizes: vec![100],
    };

    let res_doc = serde_json::to_string(&res_doc).unwrap();
    let res_doc = schema.parse_document(&res_doc).unwrap();

    index_writer.add_document(res_doc).unwrap();

    index_writer.commit()?;

    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into()?;

    let searcher = reader.searcher();

    let query_parser = QueryParser::for_index(&index, vec![file_paths]);

    let query = query_parser.parse_query("file1")?;

    let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;

    for (_score, doc_address) in top_docs {
        let retrieved_doc = searcher.doc(doc_address)?;
        println!("{}", schema.to_json(&retrieved_doc));
    }

    Ok(())
}
