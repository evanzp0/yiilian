use std::{fs, thread::sleep, time::Duration};

use yiilian_core::common::util::hash_it;

const FOLDER_NUM: u64 = 1000;

fn main() {
    let base = {
        let mut dl_path = home::home_dir().unwrap();
        dl_path.push(".yiilian/dl");

        fs::create_dir_all(dl_path.clone()).unwrap();

        dl_path
    };

    for entry in fs::read_dir(base.clone()).unwrap() {
        let entry = entry.unwrap();
        let metadata = entry.metadata().unwrap();

        if metadata.is_file() {
            
            let file_name = entry.file_name().into_string().unwrap();
            let file_main_name = file_name.split('.').collect::<Vec<&str>>()[0];
            let hash = hash_it(file_main_name);
            let mod_num = hash % FOLDER_NUM;

            let target_fd = {
                let mut tmp_path = base.clone();
                tmp_path.push(mod_num.to_string());
                fs::create_dir_all(tmp_path.clone()).unwrap();

                tmp_path.push(file_name);
                tmp_path
            };

            fs::rename(entry.path(), target_fd).unwrap();
        }

        sleep(Duration::from_secs(1));
    }
}