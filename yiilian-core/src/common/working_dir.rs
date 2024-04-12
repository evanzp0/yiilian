use std::path::PathBuf;

#[derive(Debug)]
pub struct WorkingDir {
    exec_pathes: Vec<PathBuf>,
    exec_dir: PathBuf,
    current_dir: PathBuf,
    home_dir: PathBuf,
}

impl WorkingDir {
    pub fn new() -> Self {
        let home_dir = home::home_dir().unwrap();
        std::env::set_var("HOME", home_dir.clone());
        
        let mut exec_pathes: Vec<PathBuf> = vec![];

        let exec_dir = {
            let p = std::env::current_exe().expect("Can't get the current_exe path");
            let parent = p.parent().map(|v| v.to_owned());

            parent
        };

        let mut iter_dir = exec_dir.clone();

        while let Some(path) = iter_dir.clone() {
            exec_pathes.push(path.to_path_buf());

            iter_dir = path.parent().map(|v| v.to_owned());
        }

        let current_dir = std::env::current_dir().expect("Can't get the current path");
        if !exec_pathes.contains(&current_dir) {
            exec_pathes.push(current_dir.clone());
        }

        let exec_dir = exec_dir.expect("exec_dir is None");
        WorkingDir {
            exec_pathes,
            exec_dir,
            current_dir,
            home_dir,
        }
    }

    pub fn exec_dir(&self) -> PathBuf {
        self.exec_dir.clone()
    }

    pub fn exec_pathes(&self) -> &Vec<PathBuf> {
        &self.exec_pathes
    }

    pub fn current_dir(&self) -> PathBuf {
        self.current_dir.clone()
    }

    pub fn home_dir(&self) -> PathBuf {
        self.home_dir.clone()
    }

    pub fn get_path_by_entry(&self, entry: &str) -> Option<PathBuf> {

        for path in &self.exec_pathes {
            let mut p = path.clone();
            p.push(entry);

            if p.exists() {
                return Some(p);
            }
        }

        None
    }
}

