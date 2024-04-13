
use tantivy::{schema::Schema, Index};
use yiilian_core::common::{util::setup_log4rs_from_file, working_dir::WorkingDir};
use yiilian_index::info_db_to_doc::InfoDbToDocBuilder;

#[tokio::main]
async fn main() {
    let wd = WorkingDir::new();
    let log4rs_path = wd.get_path_by_entry("log4rs.yml");
    setup_log4rs_from_file(&log4rs_path.unwrap());

    let mut db_uri = wd.current_dir();
    db_uri.push("migrations/res.db");
    let db_uri = db_uri.to_str().unwrap();
    
    let schema = Schema::builder().build();
    
    let index = Index::create_in_ram(schema.clone());

    let mut ri = InfoDbToDocBuilder::new()
        .db_uri(db_uri).await
        .index(index)
        .build();

    ri.index_loop().await;
}