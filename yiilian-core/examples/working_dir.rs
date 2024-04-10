use yiilian_core::common::{util::setup_log4rs_from_file, working_dir::WorkingDir};

fn main() {
    let wd = WorkingDir::new();
    let rst = wd.get_path_by_entry("Cargo.toml");

    println!("{:?}", rst);
    println!("{:#?}", wd.exec_pathes());

    let log4rs_path = wd.get_path_by_entry("log4rs.yml");
    println!("{:?}", log4rs_path);
    setup_log4rs_from_file(&log4rs_path.unwrap());

    log::error!("abcd");
}
