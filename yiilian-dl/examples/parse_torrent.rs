use std::fs;

use yiilian_dl::bt::data::bt_torrent::BtTorrent;

fn main() {

    let test_torrent = fs::read("/home/evan/workspace/yiilian/torrent/3.torrent").unwrap();

    let test_torrent: BtTorrent = (&test_torrent[..]).try_into().unwrap();

    println!("{:#?}", test_torrent);
}