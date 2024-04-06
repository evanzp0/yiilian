
use std::str::FromStr;

use dysql::fetch_all;
use dysql::SqlxExecutorAdatper;
use dysql::Value;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    ConnectOptions, SqliteConnection,
};

use tantivy::schema::Schema;
use tantivy::schema::STORED;
use tantivy::schema::TEXT;
use tantivy::Index;
use tantivy::IndexWriter;
use yiilian_core::common::error::Error;

use crate::res_info_doc::ResInfoDoc;
use crate::res_info_record::ResFileRecord;
use crate::res_info_record::ResInfoRecord;

pub struct InfoDbToDoc {
    db_connection: SqliteConnection,
    index_writer: IndexWriter,
    schema: Schema,
}

impl InfoDbToDoc {
    pub fn new(db_connection: SqliteConnection, index_writer: IndexWriter, schema: Schema) -> Self {
        InfoDbToDoc { db_connection, index_writer, schema }
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

    pub async fn fetch_unindex_bt_info_record(&mut self) -> Result<Vec<ResInfoRecord>, Error> {
        let mut conn = &mut self.db_connection;

        let rst = fetch_all!(|&mut conn, _| -> ResInfoRecord {
            "select * from res_info where is_indexed = 0 limit 100"
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
}


#[derive(Default)]
pub struct InfoDbToDocBuilder {
    db_connection: Option<SqliteConnection>,
    index_writer: Option<IndexWriter>,
    schema: Option<Schema>,
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

    pub fn schema(mut self, schema: Schema)-> Self {
        self.schema = Some(schema);
        self
    }

    pub fn index_writer(mut self, index_writer: IndexWriter) -> Self {
        self.index_writer = Some(index_writer);
        self
    }

    pub fn build(self) -> InfoDbToDoc {
        InfoDbToDoc::new(self.db_connection.unwrap(), self.index_writer.unwrap(), self.schema.unwrap())
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
        let index_writer = index.writer(50_000_000).unwrap();

        let mut ri = InfoDbToDocBuilder::new()
            .db_connection(conn)
            .index_writer(index_writer)
            .schema(schema)
            .build();


        let rst = ri.fetch_unindex_bt_info_record().await.unwrap();
        assert_eq!("00000000000000000001", rst[0].info_hash);

        let rst = ri.fetch_bt_files_record("00000000000000000001").await;
        assert_eq!("00000000000000000001", rst[0].info_hash);
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
                ('00000000000000000001', 0, '2024-0101T11:00:00', '2024-0101T11:00:00', 0)"
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
                ('00000000000000000001', 'file1', 100, '2024-0101T11:00:00', '2024-0101T11:00:00')"
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
