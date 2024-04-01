use std::collections::BTreeMap;

use bytes::{BufMut, Bytes, BytesMut};
use sha1::{Digest, Sha1};
use tokio::net::TcpStream;
use yiilian_core::{
    common::error::Error,
    data::{decode, BencodeData}, net::tcp::{read_bt_handshake, send_bt_handshake},
};

use crate::bt::{
    data::frame::{
        extension::{
            ExtensionHeader, UtMetadata, METADATA_PIECE_BLOCK, UT_METADATA_ID, UT_METADATA_NAME,
        },
        PeerMessage,
    },
    net::tcp::{read_message, send_message},
};

pub struct PeerWire;

impl PeerWire {
    pub fn new() -> Self {
        PeerWire
    }

    // pub async fn download_metadata(
    //     &self,
    //     info_hash: &[u8],
    //     mut stream: TcpStream,
    //     local_id: &Bytes,
    // ) -> Result<(), Error> {
    //     let hs = BtHandshake::new(&MESSAGE_EXTENSION_ENABLE, info_hash, local_id);
    //     let hs: Bytes = hs.into();
    //     stream
    //         .write_all(&hs)
    //         .await
    //         .map_err(|error| Error::new_net(Some(error.into()), None, None))?;

    //     Ok(())
    // }

    pub async fn fetch_info(
        &self,
        stream: TcpStream,
        info_hash: &[u8],
        local_peer_id: &[u8],
        is_hook: bool,
    ) -> Result<BTreeMap<Bytes, BencodeData>, Error> {
        let metadata = self
            .fetch_metdata(stream, &info_hash, &local_peer_id, is_hook)
            .await?;
        let mut info = BytesMut::new();
        info.put(&b"d4:info"[..]);
        info.extend(metadata);
        info.put(&b"e"[..]);

        decode(&info)?.as_map().map(|m| m.to_owned())
    }

    pub async fn fetch_metdata(
        &self,
        mut stream: TcpStream,
        info_hash: &[u8],
        local_peer_id: &[u8],
        is_hook: bool,
    ) -> Result<Bytes, Error> {

        if !is_hook {
            // 发送握手消息给对方
            send_bt_handshake(&mut stream, &info_hash, &local_peer_id).await?;

            // 接收对方回复的握手消息
            read_bt_handshake(&mut stream).await?;
        }

        // 发送扩展握手协议
        let ut_metadata_header = ExtensionHeader::new_ut_metadata();
        let p_msg = PeerMessage::new_ext_handshake(ut_metadata_header.into());
        let p_msg: Bytes = p_msg.into();

        send_message(&mut stream, &p_msg).await?;

        let mut pieces: Option<Vec<Bytes>> = None;
        let mut piece_num: i32 = 0;
        let mut metadata_size: usize = 0;

        loop {
            let rst = read_message(&mut stream).await?;
            let p_msg: PeerMessage = rst.try_into()?;

            match p_msg {
                // 扩展消息
                PeerMessage::Extended {
                    ext_msg_id,
                    payload,
                } => match ext_msg_id {
                    // 扩展握手消息
                    0 => {
                        if pieces != None {
                            return Err(Error::new_frame(
                                None,
                                Some(format!("recv extend message is invalid: {}, {:?}", ext_msg_id, payload)),
                            ));
                        }

                        // 从扩展 handshake 消息的中，获得 metainfo (bencoded) 大小，并检查对方是否支持 ut_metadata 扩展
                        let ext_header: ExtensionHeader = payload.try_into()?;
                        
                        if let Some(ut_metadata_id) = ext_header.get_extension_id(UT_METADATA_NAME)
                        {
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
                            return Err(Error::new_frame(
                                None,
                                Some(format!("target peer not support ut_metadata: {:?}", ext_header)),
                            ));
                        }
                    }
                    // UT_METADATA 消息
                    UT_METADATA_ID => {
                        if pieces.is_none() {
                            return Err(Error::new_frame(
                                None,
                                Some(format!("recv ut_metadata before extension handshake")),
                            ));
                        }

                        let msg: UtMetadata = payload.try_into()?;

                        match msg {
                            UtMetadata::Data {
                                piece,
                                total_size: _,
                                block,
                            } => {
                                // piece 长度校验 ， piecesNum-1 是最后一片的 piece 索引
                                let piece_len = block.len();

                                if (piece != piece_num - 1 && piece_len != METADATA_PIECE_BLOCK)
                                    || (piece == piece_num - 1
                                        && piece_len != metadata_size % METADATA_PIECE_BLOCK)
                                {
                                    return Err(Error::new_frame(
                                        None,
                                        Some(format!("recv ut_metadata piece len is invalid")),
                                    ));
                                }

                                // 将消息载荷中的 piece 数据，加入 pieces 数组
                                if let Some(pieces) = &mut pieces {
                                    pieces[piece as usize] = block;
                                }

                                // 校验 info_hash 是否有效
                                if let Some(pieces) = &pieces {
                                    if is_pieces_done(pieces) {
                                        let mut metadata_info = BytesMut::new();
                                        for item in pieces {
                                            metadata_info.extend(item);
                                        }

                                        let mut hasher = Sha1::new();
                                        hasher.update(&metadata_info);
                                        let i_hash = hasher.finalize().to_vec();

                                        if i_hash != info_hash {
                                            return Err(Error::new_frame(
                                                None,
                                                Some(format!(
                                                    "metadata info_hash is invalid: {:?}",
                                                    i_hash
                                                )),
                                            ));
                                        }

                                        // println!("metadata is downloaded");

                                        return Ok(metadata_info.into());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                },
                // PeerMessage::Bitfield {bitfield} => {
                //     let len = bitfield.len();
                //     let empty_bitfield: Bytes = {
                //         let tmp = vec![0 as u8; len];
                //         let tmp: Bytes = match tmp.try_into() {
                //             Ok(val) => val,
                //             Err(error) => return Err(Error::new_frame(Some(error.into()), None)),
                //         };

                //         tmp
                //     };
                //     let bitfield_msg = PeerMessage::Bitfield { bitfield: empty_bitfield };
                //     let bitfield_msg: Bytes = bitfield_msg.into();
            
                //     send_message(&mut stream, &bitfield_msg).await?;
                    
                //     log::trace!(target:"", "Send empty bitfield: len({len})");
                // },
                // 其他消息
                _ => {}
            }
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
    let rst = !pieces.iter().any(|item| item.len() == 0);

    rst
}
