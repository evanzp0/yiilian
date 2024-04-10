use yiilian_core::common::working_dir::WorkingDir;

fn main() {
    let wd = WorkingDir::new();
    let rst = wd.get_path_by_entry("Cargo.toml");

    println!("{:?}", rst);

    println!("{:#?}", wd.exec_pathes());
}
