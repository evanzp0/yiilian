use std::net::SocketAddr;

use bytes::{Bytes, BytesMut};
use rand::thread_rng;
use sha1::{Digest, Sha1};
use tokio::net::TcpStream;
use yiilian_core::common::error::Error;
use yiilian_crawler::{
    data::frame::{
        extension::{
            ExtensionHeader, UtMetadata, METADATA_PIECE_BLOCK, UT_METADATA_ID, UT_METADATA_NAME,
        },
        Handshake, PeerMessage,
    },
    net::tcp::{read_handshake, read_message, send_handshake, send_message},
};
use yiilian_dht::common::Id;

#[tokio::main]
async fn main() {
    let peer_address: SocketAddr = "192.168.31.6:15000".parse().unwrap();
    // let peer_address: SocketAddr = "192.168.31.6:22223".parse().unwrap();
    let info_hash = "FA84A39C18D5960B0272D3E1D2A7900FB09F5EB3";
    let info_hash = hex::decode(info_hash)
        .map_err(|hex_err| Error::new_id(Some(hex_err.into()), None))
        .unwrap();

    let peer_id = Id::from_random(&mut thread_rng()).get_bytes();

    let mut stream = TcpStream::connect(peer_address).await.unwrap();

    println!("connected");

    // 发送握手消息给对方
    send_handshake(&mut stream, &info_hash, &peer_id)
        .await
        .unwrap();

    // 接收对方回复的握手消息
    let rst = read_handshake(&mut stream).await.unwrap();

    // println!("handshake: {:?}", rst);

    // 校验对方握手消息
    if !Handshake::verify(&rst) {
        println!("recv handshake is invalid");
        return;
    }

    // 发送扩展握手协议
    let ut_metadata_header = ExtensionHeader::new_ut_metadata();
    let p_msg = PeerMessage::new_ext_handshake(ut_metadata_header.into());
    let p_msg: Bytes = p_msg.into();

    send_message(&mut stream, &p_msg).await.unwrap();

    let mut pieces: Option<Vec<Bytes>> = None;
    let mut piece_num: i32 = 0;
    let mut metadata_size: usize = 0;

    loop {
        let rst = read_message(&mut stream).await.unwrap();
        let p_msg: PeerMessage = rst.try_into().unwrap();

        match p_msg {
            // 扩展消息
            PeerMessage::Extended {
                ext_msg_id,
                payload,
            } => match ext_msg_id {
                // 扩展握手消息
                0 => {
                    if pieces != None {
                        return;
                    }

                    // 从扩展 handshake 消息的中，获得 metainfo (bencoded) 大小，并检查对方是否支持 ut_metadata 扩展
                    let ext_header: ExtensionHeader = payload.try_into().unwrap();
                    if let Some(ut_metadata_id) = ext_header.get_extension_id(UT_METADATA_NAME) {
                        if let Some(md_size) = ext_header.metadata_size {
                            if let Ok(ps_num) = request_pieces(
                                &mut stream,
                                ut_metadata_id as u8,
                                md_size as usize,
                            )
                            .await
                            {
                                piece_num = ps_num as i32;
                                metadata_size = md_size as usize;
                                pieces = Some(vec![Bytes::new(); ps_num]);

                                continue;
                            }
                        }
                    } else {
                        break;
                    }
                }
                // UT_METADATA 消息
                UT_METADATA_ID => {
                    if pieces.is_none() {
                        return
                    }

                    let msg: UtMetadata = payload.try_into().unwrap();

                    match msg {
                        UtMetadata::Data {
                            piece,
                            total_size: _,
                            block,
                        } => {
                            // piece 长度校验 ， piecesNum-1 是最后一片的 piece 索引
                            let piece_len = block.len();
                            
                            if (piece != piece_num - 1 && piece_len != METADATA_PIECE_BLOCK) 
                                || (piece == piece_num - 1 && piece_len != metadata_size % METADATA_PIECE_BLOCK)
                            {
                                return;
                            }

                            // 将消息载荷中的 piece 数据，加入 pieces 数组
                            if let Some(pieces) = &mut pieces {
                                pieces[piece as usize] = block;
                            }

                            if let Some(pieces) = &pieces {
                                if is_pieces_done(pieces) {
                                    let mut metadata_info = BytesMut::new();
                                    for item in pieces {
                                        metadata_info.extend(item);
                                    }
            
                                    let mut hasher = Sha1::new();
                                    hasher.update(metadata_info);
                                    let info = hasher.finalize().to_vec();
            
                                    if info != info_hash {
                                        println!("metadata is not valid for info_hash");
                                        return
                                    }

                                    // todo
                                    println!("metadata is downloaded");
                                    
                                    return;
                                }
                            }
                        },
                        _ => {},
                    }
                }
                _ => {}
            },
            // 其他消息
            _ => {}
        }
    }
}

/// 请求分片
async fn request_pieces(
    stream: &mut TcpStream,
    ut_metadata_id: u8,
    metadata_size: usize,
) -> Result<usize, Error> {
    // 计算元数据分片数
    let mut pieces_num = metadata_size as usize / METADATA_PIECE_BLOCK;

    if (metadata_size as usize) % METADATA_PIECE_BLOCK != 0 {
        pieces_num += 1;
    }

    for i in 0..pieces_num {
        let request = UtMetadata::Request { piece: i as i32 };
        let p_msg: Bytes = request.into_peer_message(ut_metadata_id).into();

        send_message(stream, &p_msg).await?;
    }

    Ok(pieces_num)
}

fn is_pieces_done(pieces: &Vec<Bytes>) -> bool {

    let rst = !pieces.iter().any(|item| {
        item.len() == 0
    });

    rst
}