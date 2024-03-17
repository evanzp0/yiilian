use yiilian_dht::common::Id;

#[derive(Clone, Debug)]
pub enum Event {
    CompleteDownloadBtMeta(Id)
}