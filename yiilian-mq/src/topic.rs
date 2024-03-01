use std::path::PathBuf;


pub struct Topic {
    name: String,
    path: PathBuf,
}

impl Topic {
    pub fn new(name: String, mut base_path: PathBuf) -> Self {
        base_path.push(name.clone());
        Topic {
            name,
            path: base_path,
        }
    }
}