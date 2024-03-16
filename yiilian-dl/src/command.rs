use yiilian_dht::common::Id;


#[derive(Debug)]
pub enum Command {
    DownloadBtMeta(Id),
}