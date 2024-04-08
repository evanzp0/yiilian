use std::path::Path;

use tantivy::{schema::Schema, Index};
use yiilian_index::info_db_to_doc::InfoDbToDocBuilder;

#[tokio::main]
async fn main() {
    set_up_logging_from_file::<&str>(None);

    let mut db_uri = std::env::current_dir().unwrap();
    db_uri.push("yiilian-index/migrations/res.db");
    let db_uri = db_uri.to_str().unwrap();
    let schema = Schema::builder().build();
    
    let index = Index::create_in_ram(schema.clone());

    let mut ri = InfoDbToDocBuilder::new()
        .db_uri(db_uri).await
        .index(index)
        .schema(schema)
        .build();

    ri.index_loop().await;
}

fn set_up_logging_from_file<P: AsRef<Path>>(file_path: Option<&P>) {
    if let Some(file_path) = file_path {
        log4rs::init_file(file_path, Default::default()).unwrap();
    } else {
        log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    }
}
