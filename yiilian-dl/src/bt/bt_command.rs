use yiilian_dht::common::Id;


#[derive(Debug)]
pub enum BtCommand {
    DownloadMeta(Id),
}