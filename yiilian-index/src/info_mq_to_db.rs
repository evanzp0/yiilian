
use std::fs;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use chrono::Utc;
use dysql::execute;
use dysql::SqlxExecutorAdatper;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    ConnectOptions, Connection, SqliteConnection,
};

use tokio::time::sleep;
use yiilian_core::data::MetaInfo;
use yiilian_core::{common::error::Error, data::BtTorrent};
use yiilian_mq::engine::Engine;

use crate::res_info_record::ResFileRecord;
use crate::res_info_record::ResInfoRecord;
use crate::INDEX_TOPIC_NAME;

const MQ_CLIENT_PERSIST: &str = "persist_info_client";

pub struct InfoMqToDb {
    db_connection: SqliteConnection,
    mq_engine: Arc<Mutex<Engine>>,
}

impl InfoMqToDb {
    pub fn new(db_connection: SqliteConnection, mq_engine: Arc<Mutex<Engine>>) -> Self {
        InfoMqToDb { db_connection, mq_engine }
    }

    pub async fn persist_loop(&mut self) {
        loop {
            let message = self.mq_engine.lock().expect("lock mq_engine").poll_message(INDEX_TOPIC_NAME, MQ_CLIENT_PERSIST);
            if let Some(message) = message {
                let meta_path = unsafe { String::from_utf8_unchecked(message.value().into()) };
                match fs::read(meta_path) {
                    Ok(val) => {
                        match BtTorrent::try_from(&val[..]) {
                            Ok(bt_torrent) => {
                                if let Err(error) = self.add_bt_info_record(&bt_torrent).await {
                                    log::trace!(target: "yiilian_index::info_mq_to_db::persist_loop", "add_bt_info_record error: {}", error);
                                } else {
                                    log::trace!(target: "yiilian_index::info_mq_to_db::persist_loop", "persisted bt: {}", bt_torrent.info_hash);
                                }
                            },
                            Err(error) => {
                                log::trace!(target: "yiilian_index::info_mq_to_db::persist_loop", "Decode bt_torrent error: {}", error);
                            },
                        }
                    },
                    Err(error) => {
                        log::trace!(target: "yiilian_index::info_mq_to_db::persist_loop", "Read bt_torrent error: {}", error);
                    },
                }
            }

            sleep(Duration::from_secs(1)).await;
        }
    }

    pub async fn add_bt_info_record(&mut self, bt_torrent: &BtTorrent) -> Result<(), Error> {
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

        let dto = ResInfoRecord {
            info_hash: bt_torrent.info_hash.clone(),
            res_type: 1,
            create_time: now.clone(),
            mod_time: now.clone(),
            is_indexed: 0,
        };

        let conn = &mut self.db_connection;
        let mut tran = conn
            .begin()
            .await
            .map_err(|error| Error::new_db(Some(error.into()), None))?;

        let _ = execute!(|&mut *tran, dto| {r#"
            insert into res_info
                (info_hash, res_type, create_time, mod_time, is_indexed) 
            values 
                (:info_hash, :res_type,:create_time, :mod_time, :is_indexed)
        "#})
        .map_err(|error| Error::new_db(Some(error.into()), None))?;

        let mut res_files = vec![];

        match &bt_torrent.info {
            MetaInfo::SingleFile { length, name, .. } => {
                let file = ResFileRecord {
                    info_hash: bt_torrent.info_hash.clone(),
                    file_path: name.clone(),
                    file_size: *length,
                    create_time: now.clone(),
                    mod_time: now.clone(),
                };

                res_files.push(file);
            }
            MetaInfo::MultiFile { files, .. } => {
                for f in files {
                    let file = ResFileRecord {
                        info_hash: bt_torrent.info_hash.clone(),
                        file_path: f.path.clone(),
                        file_size: f.length,
                        create_time: now.clone(),
                        mod_time: now.clone(),
                    };

                    res_files.push(file);
                }
            }
        }

        for res_file in res_files {
            let _ = execute!(|&mut *tran, res_file| {r#"
                insert into res_file
                    (info_hash, file_path, file_size, create_time, mod_time)
                values 
                    (:info_hash, :file_path, :file_size, :create_time, :mod_time)
            "#})
            .map_err(|error| Error::new_db(Some(error.into()), None))?;
        }

        tran.commit()
            .await
            .map_err(|error| Error::new_db(Some(error.into()), None))?;

        Ok(())
    }
}


#[derive(Default)]
pub struct InfoMqToDbBuilder {
    db_connection: Option<SqliteConnection>,
    mq_engine: Option<Arc<Mutex<Engine>>>,
}

impl InfoMqToDbBuilder {
    pub fn new() -> InfoMqToDbBuilder {
        InfoMqToDbBuilder::default()
    }

    pub async fn db_uri(mut self, db_uri: &str) -> Self {
        let db_connection = SqliteConnectOptions::from_str(db_uri)
            .unwrap()
            .journal_mode(SqliteJournalMode::Wal)
            .read_only(false)
            .connect()
            .await
            .unwrap();

        self.db_connection = Some(db_connection);

        self
    }

    pub fn db_connection(mut self, db_connection: SqliteConnection) -> Self {
        self.db_connection = Some(db_connection);
        self
    }

    pub fn mq_engine(mut self, mq_engine: Arc<Mutex<Engine>>) -> Self {
        self.mq_engine = Some(mq_engine);
        self
    }

    pub fn build(self) -> InfoMqToDb {
        InfoMqToDb::new(self.db_connection.unwrap(), self.mq_engine.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use yiilian_core::{common::shutdown::create_shutdown, data::{FileInfo, MetaInfo}};
    use yiilian_mq::segment::LOG_DATA_SIZE;

    use super::*;

    #[tokio::test]
    async fn test_add_single_and_fetch() {
        let (mut _shutdown_tx, shutdown_rx) = create_shutdown();

        let mut mq_engine = Engine::new(LOG_DATA_SIZE, shutdown_rx).unwrap();
        mq_engine.open_topic("test_info_mq").unwrap();
        let mq_engine = Arc::new(Mutex::new(mq_engine));

        let conn = connect_db().await;

        let mut ri= InfoMqToDbBuilder::new()
            .db_connection(conn)
            .mq_engine(mq_engine.clone())
            .build();

        let info_hash = "00000000000000000001".to_owned();

        let bt_torrent = BtTorrent {
            info_hash: info_hash.clone(),
            announce: "".to_owned(),
            info: MetaInfo::SingleFile {
                length: 1200,
                name: "test_file".to_owned(),
                pieces: b"pieces"[..].into(),
                piece_length: 1000,
            },
        };

        ri.add_bt_info_record(&bt_torrent).await.unwrap();
        mq_engine.lock().expect("lock mq_engine").remove_topic("test_info_mq");
    }

    #[tokio::test]
    async fn test_add_multiple() {
        let (mut _shutdown_tx, shutdown_rx) = create_shutdown();

        let mut mq_engine = Engine::new(LOG_DATA_SIZE, shutdown_rx).unwrap();
        mq_engine.open_topic("test_info_mq1").unwrap();
        let mq_engine = Arc::new(Mutex::new(mq_engine));

        let conn = connect_db().await;

        let mut ri= InfoMqToDbBuilder::new()
            .db_connection(conn)
            .mq_engine(mq_engine.clone())
            .build();

        let mf = MetaInfo::MultiFile {
            files: vec![
                FileInfo {
                    length: 100,
                    path: "f1".to_owned(),
                },
                FileInfo {
                    length: 200,
                    path: "f2".to_owned(),
                },
            ],
            name: "test_mf".to_owned(),
            pieces: b"pieces"[..].into(),
            piece_length: 1000,
        };

        let bt_torrent = BtTorrent {
            info_hash: "00000000000000000001".to_owned(),
            announce: "".to_owned(),
            info: mf,
        };

        ri.add_bt_info_record(&bt_torrent).await.unwrap();
        mq_engine.lock().expect("lock mq_engine").remove_topic("test_info_mq1");
    }

    async fn connect_db() -> sqlx::SqliteConnection {
        let mut conn = sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .read_only(false)
            .connect()
            .await
            .unwrap();

        // prepare table scehma
        sqlx::query("DROP TABLE IF EXISTS res_info")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query(
            r#"
            CREATE TABLE res_info (
                info_hash VARCHAR(100) PRIMARY KEY,
                res_type INT NOT NULL,
                create_time VARCHAR(100) NOT NULL,
                mod_time VARCHAR(100) NOT NULL,
                is_indexed INT NOT NULL
            )"#,
        )
        .execute(&mut conn)
        .await
        .unwrap();

        sqlx::query("DROP TABLE IF EXISTS res_file")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query(
            r#"
            CREATE TABLE res_file (
                info_hash VARCHAR(100) NOT NULL,
                file_path VARCHAR(1000) NOT NULL,
                file_size INT NOT NULL,
                create_time VARCHAR(100) NOT NULL,
                mod_time VARCHAR(100) NOT NULL
            )"#,
        )
        .execute(&mut conn)
        .await
        .unwrap();

        sqlx::query("DROP INDEX IF EXISTS idx_res_file_info_hash")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("CREATE INDEX idx_res_file_info_hash ON res_file (info_hash)")
            .execute(&mut conn)
            .await
            .unwrap();
        conn
    }
}
