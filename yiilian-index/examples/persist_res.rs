use std::sync::{Arc, Mutex};

use yiilian_core::common::working_dir::WorkingDir;
use yiilian_index::info_mq_to_db::InfoMqToDbBuilder;
use yiilian_mq::{engine::Engine, segment::LOG_DATA_SIZE};


#[tokio::main]
async fn main() {
    let wd = WorkingDir::new();
    // let log4rs_path = wd.get_path_by_entry("log4rs.yml");
    // setup_log4rs_from_file(&log4rs_path.unwrap());

    let mut mq_engine = Engine::new(LOG_DATA_SIZE, wd.home_dir()).unwrap();
    mq_engine.open_topic("info_index").unwrap();
    let mq_engine = Arc::new(Mutex::new(mq_engine));
    
    let db_uri = wd.home_dir().join(".yiilian/db/res.db");
    let db_uri = db_uri.to_str().unwrap();

    // let db_uri = "/home/evan/workspace/yiilian/yiilian-index/migrations/res.db";

    let mut mqdb = InfoMqToDbBuilder::new()
        .db_uri(db_uri).await
        .mq_engine(mq_engine)
        .build();

    mqdb.persist_loop().await;
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
