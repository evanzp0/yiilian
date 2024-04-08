use std::{path::Path, sync::Arc};

use yiilian_core::common::shutdown::create_shutdown;
use yiilian_index::info_mq_to_db::InfoMqToDbBuilder;
use yiilian_mq::{engine::Engine, segment::LOG_DATA_SIZE};


#[tokio::main]
async fn main() {
    set_up_logging_from_file::<&str>(None);

    let (mut _shutdown_tx, shutdown_rx) = create_shutdown();

    let mut mq_engine = Engine::new(LOG_DATA_SIZE, shutdown_rx).unwrap();
    mq_engine.open_topic("info_index").unwrap();
    let mq_engine = Arc::new(mq_engine);
    
    let mut db_uri = std::env::current_dir().unwrap();
    db_uri.push("yiilian-index/migrations/res.db");
    let db_uri = db_uri.to_str().unwrap();

    // let db_uri = "/home/evan/workspace/yiilian/yiilian-index/migrations/res.db";

    let mut mqdb = InfoMqToDbBuilder::new()
        .db_uri(db_uri).await
        .mq_engine(mq_engine)
        .build();

    mqdb.persist_loop().await;
}


fn set_up_logging_from_file<P: AsRef<Path>>(file_path: Option<&P>) {
    if let Some(file_path) = file_path {
        log4rs::init_file(file_path, Default::default()).unwrap();
    } else {
        log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    }
}


#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use sqlx::{
        sqlite::{SqliteConnectOptions, SqliteJournalMode},
        ConnectOptions, 
    };
    
    #[tokio::test]

    async fn test_open_db() {
        let db_uri = "/home/evan/workspace/yiilian/yiilian-index/migrations/res.db";
    
        let _db_connection = SqliteConnectOptions::from_str(db_uri)
            .unwrap()
            .journal_mode(SqliteJournalMode::Wal)
            .read_only(false)
            .connect()
            .await
            .unwrap();
        }
}
