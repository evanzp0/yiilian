use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use dysql::execute;
use dysql::fetch_all;
use dysql::SqlxExecutorAdatper;
use dysql::Value;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    ConnectOptions, SqliteConnection,
};

use tantivy::schema::Schema;
use tantivy::schema::INDEXED;
use tantivy::schema::STORED;
use tantivy::schema::STRING;
use tantivy::schema::TEXT;
use tantivy::Index;
use tokio::time::sleep;
use yiilian_core::common::error::Error;

use crate::res_info_doc::ResInfoDoc;
use crate::res_info_record::ResFileRecord;
use crate::res_info_record::ResInfoRecord;

const MAX_PROC_DOC_NUM: i32 = 1000;
const INDEX_INTERVAL_SEC: u64 = 60 * 60;
const INDEX_WRITER_BUF_SIZE: usize = 50_000_000;

pub struct InfoDbToDoc {
    db_connection: SqliteConnection,
    index: Index,
}

impl InfoDbToDoc {
    pub fn new(db_connection: SqliteConnection, index: Index) -> Self {
        InfoDbToDoc {
            db_connection,
            index,
        }
    }

    pub async fn index_loop(&mut self) {
        let mut proc_doc_num = 0;
        let mut is_found = false;
        loop {
            sleep(Duration::from_secs(1)).await;
            let fetch_rst = self.fetch_unindex_bt_info_record().await;

            match fetch_rst {
                Err(error) => {
                    log::trace!(target: "yiilian_index::info_db_to_doc::index_loop", "fetch_rst error: {}", error);
                    continue;
                }
                Ok(res_infos) => {
                    if res_infos.len() > 0 {
                        is_found = true;

                        for res_info in res_infos {
                            let fetch_files_rst =
                                self.fetch_bt_files_record(&res_info.info_hash).await;
                            match fetch_files_rst {
                                Err(error) => {
                                    log::trace!(target: "yiilian_index::info_db_to_doc::index_loop", "fetch_files_rst error: {}", error);
                                    break;
                                }
                                Ok(res_files) => {
                                    if let Err(error) = self.index_res_info(&res_info, &res_files) {
                                        log::trace!(target: "yiilian_index::info_db_to_doc::index_loop", "index_res_info error: {}", error);
                                        continue;
                                    } else {
                                        log::trace!(target: "yiilian_index::info_db_to_doc::index_loop", "index info: {}", res_info.info_hash);

                                        self.update_indexed_res_info(&res_info.info_hash)
                                            .await
                                            .ok();
                                    }
                                }
                            }

                            proc_doc_num += 1;
                        }
                    }
                }
            }

            if !is_found || proc_doc_num >= MAX_PROC_DOC_NUM {
                proc_doc_num = 0;
                is_found = false;

                let segments = self
                    .index
                    .searchable_segment_ids()
                    .expect("searchable_segment_ids");

                if segments.len() > 0 {
                    let mut index_writer = self
                        .index
                        .writer(INDEX_WRITER_BUF_SIZE)
                        .expect("get index_writer");
                    index_writer.merge(&segments);

                    index_writer.wait_merging_threads().expect("wait_merging_threads");

                    log::trace!(target: "yiilian_index::info_db_to_doc::index_loop", "Merged segments: {}", segments.len());
                }
                sleep(Duration::from_secs(INDEX_INTERVAL_SEC)).await;
            }
        }
    }

    async fn update_indexed_res_info(&mut self, info_hash: &str) -> Result<(), Error> {
        let mut conn = &mut self.db_connection;

        let value = Value::new(info_hash);

        execute!(|&mut conn, &value| {
            "update  res_info set is_indexed = 1 WHERE info_hash = :value"
        })
        .map_err(|error| Error::new_db(Some(error.into()), None))?;

        Ok(())
    }

    pub fn index_res_info(
        &mut self,
        res_info: &ResInfoRecord,
        res_files: &Vec<ResFileRecord>,
    ) -> Result<(), Error> {
        let mut file_paths_value = vec![];
        let mut file_sizes_value = vec![];
        for file in res_files {
            // file_paths_value.push(serde_json::Value::from(file.file_path.clone()));
            // file_sizes_value.push(serde_json::Value::from(file.file_size));
            file_paths_value.push(file.file_path.clone());
            file_sizes_value.push(file.file_size);
        }

        // let file_paths_value = serde_json::Value::Array(file_paths_value);
        // let file_sizes_value = serde_json::Value::Array(file_sizes_value);

        // let schema = self.index.schema();

        // let info_hash = schema.get_field("info_hash").unwrap();
        // let res_type = schema.get_field("res_type").unwrap();
        // let create_time = schema.get_field("create_time").unwrap();
        // let file_paths = schema.get_field("file_paths").unwrap();
        // let file_sizes = schema.get_field("file_sizes").unwrap();

        // let res_doc = doc! {
        //     info_hash => res_info.info_hash.clone(),
        //     res_type =>  res_info.res_type as u64,
        //     create_time => res_info.create_time.clone(),
        //     file_paths => file_paths_value,
        //     file_sizes=> file_sizes_value,
        // };

        let res_doc = ResInfoDoc {
            info_hash: res_info.info_hash.clone(),
            res_type: res_info.res_type,
            create_time: res_info.create_time.clone(),
            file_paths: file_paths_value,
            file_sizes: file_sizes_value,
        };
        let res_doc = serde_json::to_string(&res_doc)
            .map_err(|error| Error::new_index(Some(error.into()), None))?;
        let res_doc = self.index
            .schema()
            .parse_document(&res_doc)
            .map_err(|error| Error::new_index(Some(error.into()), None))?;

        let mut index_writer = self.index.writer(INDEX_WRITER_BUF_SIZE).unwrap();

        index_writer
            .add_document(res_doc)
            .map_err(|error| Error::new_index(Some(error.into()), None))?;

        index_writer
            .commit()
            .map_err(|error| Error::new_index(Some(error.into()), None))?;

        Ok(())
    }

    pub async fn fetch_unindex_bt_info_record(&mut self) -> Result<Vec<ResInfoRecord>, Error> {
        let mut conn = &mut self.db_connection;

        let rst = fetch_all!(|&mut conn, _| -> ResInfoRecord {
            "select * from res_info where is_indexed = 0 limit 100"
        })
        .map_err(|error| Error::new_db(Some(error.into()), None))?;

        Ok(rst)
    }

    pub async fn fetch_bt_files_record(
        &mut self,
        info_hash: &str,
    ) -> Result<Vec<ResFileRecord>, Error> {
        let mut conn = &mut self.db_connection;
        let value = Value::new(info_hash);

        let rst = fetch_all!(|&mut conn, &value| -> ResFileRecord {
            "SELECT * FROM res_file WHERE info_hash = :value"
        })
        .map_err(|error| Error::new_db(Some(error.into()), None))?;

        Ok(rst)
    }
}

#[derive(Default)]
pub struct InfoDbToDocBuilder {
    db_connection: Option<SqliteConnection>,
    index: Option<Index>,
}

impl InfoDbToDocBuilder {
    pub fn new() -> InfoDbToDocBuilder {
        InfoDbToDocBuilder::default()
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

    pub fn index_path(mut self, index_path: PathBuf) -> Self {
        let index = match Index::open_in_dir(&index_path) {
            Ok(val) => val,
            Err(_) => {
                let mut schema_builder = Schema::builder();
                schema_builder.add_text_field("info_hash", STRING | STORED);
                schema_builder.add_u64_field("res_type", INDEXED | STORED);
                schema_builder.add_text_field("create_time", STORED);
                schema_builder.add_text_field("file_paths", TEXT | STORED);
                schema_builder.add_u64_field("file_sizes", STORED);
        
                let schema = schema_builder.build();
    
                Index::create_in_dir(&index_path, schema.clone()).unwrap()
            },
        };

        self.index = Some(index);

        self
    }

    pub fn db_connection(mut self, db_connection: SqliteConnection) -> Self {
        self.db_connection = Some(db_connection);
        self
    }

    pub fn index(mut self, index: Index) -> Self {
        self.index = Some(index);
        self
    }

    pub fn build(self) -> InfoDbToDoc {
        InfoDbToDoc::new(
            self.db_connection.unwrap(),
            self.index.unwrap(),
        )
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_add_single_and_fetch() {
        let conn = connect_db().await;
        let schema = Schema::builder().build();
        let index = Index::create_in_ram(schema.clone());

        let mut ri = InfoDbToDocBuilder::new()
            .db_connection(conn)
            .index(index)
            .build();

        let rst = ri.fetch_unindex_bt_info_record().await.unwrap();
        assert_eq!("00000000000000000001", rst[0].info_hash);

        let rst = ri.fetch_bt_files_record("00000000000000000001").await;
        assert_eq!("00000000000000000001", rst.unwrap()[0].info_hash);
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

        sqlx::query(
            "insert into res_info 
                (info_hash, res_type, create_time, mod_time, is_indexed)
            values
                ('00000000000000000001', 0, '2024-0101T11:00:00', '2024-0101T11:00:00', 0)",
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

        sqlx::query(
            "insert into res_file 
                (info_hash, file_path, file_size, create_time, mod_time)
            values
                ('00000000000000000001', 'file1', 100, '2024-0101T11:00:00', '2024-0101T11:00:00')",
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
