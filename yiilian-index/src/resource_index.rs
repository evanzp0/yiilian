
use std::str::FromStr;

use chrono::Utc;
use dysql::execute;
use dysql::fetch_all;
use dysql::page;
use dysql::PageDto;
use dysql::Pagination;
use dysql::SqlxExecutorAdatper;
use dysql::Value;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    ConnectOptions, Connection, SqliteConnection,
};

use tantivy::schema::Schema;
use tantivy::schema::STORED;
use tantivy::schema::TEXT;
use tantivy::Index;
use tantivy::IndexWriter;
use yiilian_core::data::MetaInfo;
use yiilian_core::{common::error::Error, data::BtTorrent};

use crate::res_info_doc::ResInfoDoc;
use crate::res_info_record::ResFileRecord;
use crate::res_info_record::ResInfoRecord;

pub struct ResourceIndex {
    db_connection: SqliteConnection,
    index_writer: IndexWriter,
    schema: Schema,
}

impl ResourceIndex {
    pub fn new(db_connection: SqliteConnection, index_writer: IndexWriter, schema: Schema) -> Self {
        ResourceIndex { db_connection, index_writer, schema }
    }

    pub fn index_res_info(&mut self, res_info: &ResInfoRecord, res_files: &Vec<ResFileRecord>) -> Result<(), Error> {

        let mut file_paths = vec![];
        let mut file_sizes = vec![];
        for file in  res_files {
            file_paths.push(file.file_path.clone());
            file_sizes.push(file.file_size);
        }

        let res_doc = ResInfoDoc {
            info_hash: res_info.info_hash.clone(),
            res_type: res_info.res_type,
            create_time: res_info.create_time.clone(),
            file_paths,
            file_sizes,
        };

        let res_doc = serde_json::to_string(&res_doc)
            .map_err(|error| Error::new_index(Some(error.into()), None))?;
        let res_doc = self.schema.parse_document(&res_doc)
            .map_err(|error| Error::new_index(Some(error.into()), None))?;

        self.index_writer.add_document(res_doc)
            .map_err(|error| Error::new_index(Some(error.into()), None))?;

        self.index_writer.commit()
            .map_err(|error| Error::new_index(Some(error.into()), None))?;

        Ok(())
    }

    pub async fn fetch_unindex_bt_info_record(
        &mut self,
        page_no: u64,
    ) -> Result<Pagination<ResInfoRecord>, Error> {
        let mut pg_dto = PageDto::new_with_sort(100, page_no, Option::<()>::None, vec![]);

        let mut conn = &mut self.db_connection;

        let rst = page!(|&mut conn, pg_dto| -> ResInfoRecord {
            "select * from res_info where is_indexed = 0"
        })
        .map_err(|error| Error::new_db(Some(error.into()), None))?;

        Ok(rst)
    }

    pub async fn fetch_bt_files_record(&mut self, info_hash: &str) -> Vec<ResFileRecord> {
        let mut conn = &mut self.db_connection;
        let value = Value::new(info_hash);

        let rst = fetch_all!(|&mut conn, &value| -> ResFileRecord {
            "SELECT * FROM res_file WHERE info_hash = :value"
        })
        .unwrap();

        rst
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
pub struct ResourceIndexBuilder {
    db_connection: Option<SqliteConnection>,
    index_writer: Option<IndexWriter>,
    schema: Option<Schema>,
}

impl ResourceIndexBuilder {
    pub fn new() -> ResourceIndexBuilder {
        ResourceIndexBuilder::default()
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

    pub fn index_path(mut self, index_path: &str) -> Self {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("info_hash", TEXT | STORED);
        schema_builder.add_i64_field("res_type", STORED);
        schema_builder.add_text_field("create_time", STORED);
        schema_builder.add_text_field("file_paths", TEXT | STORED);
        schema_builder.add_i64_field("file_sizes", STORED);
    
        let schema = schema_builder.build();
    
        let index = Index::create_in_dir(&index_path, schema.clone()).unwrap();
        let index_writer = index.writer(50_000_000).unwrap();

        self.index_writer = Some(index_writer);
        self.schema = Some(schema);

        self
    }

    pub fn db_connection(mut self, db_connection: SqliteConnection) -> Self {
        self.db_connection = Some(db_connection);
        self
    }

    pub fn index_writer(mut self, index_writer: IndexWriter) -> Self {
        self.index_writer = Some(index_writer);
        self
    }

    pub fn build(self) -> ResourceIndex {
        ResourceIndex::new(self.db_connection.unwrap(), self.index_writer.unwrap(), self.schema.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use yiilian_core::data::{FileInfo, MetaInfo};

    use super::*;

    #[tokio::test]
    async fn test_add_single_and_fetch() {
        let conn = connect_db().await;
        let schema = Schema::builder().build();
        let index = Index::create_in_ram(schema.clone());
        let index_writer = index.writer(50_000_000).unwrap();

        let mut ri= ResourceIndexBuilder::new()
            .db_connection(conn)
            .index_writer(index_writer)
            .build();

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

        let rst = ri.fetch_unindex_bt_info_record(0).await.unwrap();
        assert_eq!("00000000000000000001", rst.data[0].info_hash);

        let rst = ri.fetch_bt_files_record("00000000000000000001").await;
        assert_eq!("00000000000000000001", rst[0].info_hash);
    }

    #[tokio::test]
    async fn test_add_multiple() {
        let conn = connect_db().await;
        let schema = Schema::builder().build();
        let index = Index::create_in_ram(schema.clone());
        let index_writer = index.writer(50_000_000).unwrap();

        let mut ri= ResourceIndexBuilder::new()
            .db_connection(conn)
            .index_writer(index_writer)
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
