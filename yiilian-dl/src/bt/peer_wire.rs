use std::{collections::BTreeMap, net::SocketAddr};

use bytes::{BufMut, Bytes, BytesMut};
use rand::thread_rng;
use sha1::{Digest, Sha1};
use tokio::{io::AsyncWriteExt, net::TcpStream};
use yiilian_core::{
    common::error::Error,
    data::{decode, BencodeData},
};
use yiilian_dht::common::Id;

use crate::bt::{
    data::frame::{
        extension::{
            ExtensionHeader, UtMetadata, METADATA_PIECE_BLOCK, UT_METADATA_ID, UT_METADATA_NAME,
        },
        Handshake, PeerMessage, MESSAGE_EXTENSION_ENABLE,
    },
    net::tcp::{read_handshake, read_message, send_handshake, send_message},
};

pub struct PeerWire;

impl PeerWire {
    pub fn new() -> Self {
        PeerWire
    }

    pub async fn download_metadata(
        &self,
        info_hash: &[u8],
        peer_address: SocketAddr,
    ) -> Result<(), Error> {
        let mut stream = TcpStream::connect(peer_address)
            .await
            .map_err(|error| Error::new_net(Some(error.into()), None, Some(peer_address)))?;

        let peer_id = Id::from_random(&mut thread_rng()).get_bytes();
        let hs = Handshake::new(&MESSAGE_EXTENSION_ENABLE, info_hash, &peer_id);
        let hs: Bytes = hs.into();
        stream
            .write_all(&hs)
            .await
            .map_err(|error| Error::new_net(Some(error.into()), None, Some(peer_address)))?;

        Ok(())
    }

    pub async fn fetch_info(
        &self,
        target_address: SocketAddr,
        info_hash: &[u8],
        local_peer_id: &[u8],
    ) -> Result<BTreeMap<Bytes, BencodeData>, Error> {
        let metadata = self
            .fetch_metdata(target_address, &info_hash, &local_peer_id)
            .await?;
        let mut info = BytesMut::new();
        info.put(&b"d4:info"[..]);
        info.extend(metadata);
        info.put(&b"e"[..]);

        decode(&info)?.as_map().map(|m| m.to_owned())
    }

    pub async fn fetch_metdata(
        &self,
        target_address: SocketAddr,
        info_hash: &[u8],
        local_peer_id: &[u8],
    ) -> Result<Bytes, Error> {
        let mut stream = TcpStream::connect(target_address)
            .await
            .map_err(|err| Error::new_net(Some(err.into()), None, Some(target_address)))?;

        // 发送握手消息给对方
        send_handshake(&mut stream, &info_hash, &local_peer_id).await?;

        // 接收对方回复的握手消息
        let rst = read_handshake(&mut stream).await.map_err(|error| Error::new_net(Some(error.into()), None, Some(target_address)))?;

        // 校验对方握手消息
        if !Handshake::verify(&rst) {
            return Err(Error::new_frame(
                None,
                Some(format!("recv handshake is invalid: {:?}", rst)),
            ));
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
                                Some(format!("recv extend message is invalid")),
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
                                Some(format!("recv extend message is invalid")),
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
                                            println!("metadata is not valid for info_hash");
                                            return Err(Error::new_frame(
                                                None,
                                                Some(format!(
                                                    "metadata info_hash is invalid: {:?}",
                                                    i_hash
                                                )),
                                            ));
                                        }

                                        // todo
                                        println!("metadata is downloaded");

                                        return Ok(metadata_info.into());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                },
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
