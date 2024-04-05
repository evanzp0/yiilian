use dysql::Content;
use sqlx::FromRow;


#[derive(FromRow, Content, Clone, Debug)]
pub struct ResInfo {
    pub info_hash: String,
    pub res_type: u8,
    pub create_time: String,
    pub mod_time: String,
    pub is_indexed: u8,
} 

#[derive(FromRow, Content, Clone, Debug)]
pub struct ResFile {
    pub info_hash: String,
    pub file_path: String,
    pub file_size: usize,
    pub create_time: String,
    pub mod_time: String,
}