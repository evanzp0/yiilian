
#[derive(Clone, Debug)]
pub struct ResInfoDoc {
    pub info_hash: String,
    pub res_type: i32,
    pub create_time: String,
    pub files: Vec<ResFileDoc>
}

#[derive(Clone, Debug)]
pub struct ResFileDoc {
    pub file_path: String,
    pub file_size: i64,
}