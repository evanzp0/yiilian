use std::str::FromStr;

use chrono::Utc;
use dysql::execute;
use dysql::SqlxExecutorAdatper;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    ConnectOptions, SqliteConnection,
};

use yiilian_core::{common::error::Error, data::BtTorrent};

use crate::res_info::ResInfo;

pub struct ResourceIndex {
    db_connection: SqliteConnection,
}

impl ResourceIndex {
    pub async fn new(db_uri: &str) -> Result<Self, Error> {
        let db_connection = SqliteConnectOptions::from_str(db_uri)
            .map_err(|error| Error::new_db(Some(error.into()), None))?
            .journal_mode(SqliteJournalMode::Wal)
            .read_only(false)
            .connect()
            .await
            .map_err(|error| Error::new_db(Some(error.into()), None))?;

        Ok(ResourceIndex { db_connection })
    }

    pub fn new_from_conn(db_connection: SqliteConnection) -> Self {
        ResourceIndex { db_connection }
    }

    pub async fn add_bt_info_record(&mut self, bt_torrent: &BtTorrent) -> Result<(), Error> {
        let conn = &mut self.db_connection;
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

        let dto = ResInfo {
            info_hash: bt_torrent.info_hash.clone(),
            res_type: 1,
            create_time: now.clone(),
            mod_time: now,
            is_indexed: 0,
        };

        println!("{:?}", dto);

        let _ = execute!(|&mut *conn, dto| {
            r#"insert into res_info
                    (info_hash, res_type, create_time, mod_time, is_indexed) 
                values 
                    (:info_hash, :res_type,:create_time, :mod_time, :is_indexed)"#
        })
        .map_err(|error| Error::new_db(Some(error.into()), None))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use yiilian_core::data::MetaInfo;

    use super::*;

    #[tokio::test]
    async fn test_db() {
        let conn = connect_db().await;

        let mut ri = ResourceIndex::new_from_conn(conn);

        let bt_torrent = BtTorrent {
            info_hash: "00000000000000000001".to_owned(),
            announce: "".to_owned(),
            info: MetaInfo::SingleFile {
                length: 1200,
                name: "test_file".to_owned(),
                pieces: b"pieces"[..].into(),
                piece_length: 1000,
            },
        };

        ri.add_bt_info_record(&bt_torrent).await.unwrap();
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

        // sqlx::query("DROP TABLE IF EXISTS res_file")
        //     .execute(&mut conn)
        //     .await
        //     .unwrap();
        // sqlx::query(
        //     r#"
        //     CREATE TABLE res_file (
        //         info_hash VARCHAR(100) NOT NULL,
        //         file_path VARCHAR(1000) NOT NULL,
        //         file_size INT NOT NULL,
        //         create_time VARCHAR(100) NOT NULL,
        //         mod_time VARCHAR(100) NOT NULL
        //     )"#,
        // )
        // .execute(&mut conn)
        // .await
        // .unwrap();

        conn
    }
}
