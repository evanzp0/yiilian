use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResInfoDoc {
    pub info_hash: String,
    pub res_type: i32,
    pub create_time: String,
    pub file_paths: Vec<String>,
    pub file_sizes: Vec<i32>,
}
