use std::str::FromStr;

use chrono::Utc;
use dysql::execute;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    ConnectOptions, SqliteConnection,
};
use dysql::SqlxExecutorAdatper;

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

    pub async fn db_save_bt_info(&mut self, bt_torrent: &BtTorrent) -> Result<(), Error> {
        let conn = &mut self.db_connection;
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

        let dto = ResInfo {
            info_hash: bt_torrent.info_hash.clone(),
            res_type: 1,
            create_time: now.clone(),
            mod_time: now,
            is_indexed: 0,
        };

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
