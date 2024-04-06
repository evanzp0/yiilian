use dysql::Content;
use sqlx::FromRow;


#[derive(FromRow, Content, Clone, Debug)]
pub struct ResInfoRecord {
    pub info_hash: String,
    pub res_type: i32,
    pub create_time: String,
    pub mod_time: String,
    pub is_indexed: i32,
} 

#[derive(FromRow, Content, Clone, Debug)]
pub struct ResFileRecord {
    pub info_hash: String,
    pub file_path: String,
    pub file_size: i64,
    pub create_time: String,
    pub mod_time: String,
}