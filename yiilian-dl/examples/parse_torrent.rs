use std::fs;

use yiilian_core::data::BtTorrent;

fn main() {

    let test_torrent = fs::read("/home/evan/workspace/yiilian/torrent/4fd706a0c360cd1c447741ae2b921bfc7ea814ef.torrent").unwrap();

    let test_torrent: BtTorrent = (&test_torrent[..]).try_into().unwrap();

    println!("{:#?}", test_torrent);
}