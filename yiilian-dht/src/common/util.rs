use std::net::SocketAddr;

use crate::routing_table::Node;

use crc::{Crc, CRC_32_ISCSI};
use yiilian_core::common::{error::Error, util::{bytes_to_sockaddr, sockaddr_to_bytes}};

use super::id::Id;

pub const CASTAGNOLI: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);

/// Calculates a peer announce token based on a sockaddr and some secret.
/// Pretty positive this isn't cryptographically safe but I'm not too worried.
/// If we care about that later we can use a proper HMAC or something.
pub fn calculate_token<T: AsRef<[u8]>>(remote: &SocketAddr, secret: T) -> [u8; 4] {
    let secret = secret.as_ref();
    let mut digest = CASTAGNOLI.digest();
    
    let octets = match remote.ip() {
        std::net::IpAddr::V4(v4) => v4.octets().to_vec(),
        std::net::IpAddr::V6(v6) => v6.octets().to_vec(),
    };
    digest.update(&octets);
    digest.update(secret);
    let checksum: u32 = digest.finalize();

    checksum.to_be_bytes()
}


/// 紧凑格式转 Node 数组 (node_id + ip + port)
/// 
/// # Example
/// ```
/// # use yiilian_dht::routing_table::*;
/// # use yiilian_dht::util::*;
/// # use yiilian_dht::common::*;
/// use bytes::Bytes;
/// use std::net::{SocketAddr, ToSocketAddrs};
/// 
/// let data: Bytes = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1, 127,0,0,1, 0,80].into();
/// let rst = bytes_to_nodes4(&data, 20).unwrap();
/// let id = Id::from_bytes(&vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1]);
/// let sock_addr: SocketAddr = "127.0.0.1:80".parse().unwrap();
/// let node = Node::new(id, sock_addr);
/// assert_eq!(node, rst[0]);
/// ```
pub fn bytes_to_nodes4(bytes: &[u8], id_size: usize) -> Result<Vec<Node>, Error> {
    let bytes = bytes.as_ref();
    let node4_byte_size: usize = id_size + 6;
    if bytes.len() % node4_byte_size != 0 {
        Err(Error::new_frame(None, Some(format!("Wrong number of bytes for nodes message ({})", bytes.len()))))?;
    }

    let expected_num = bytes.len() / node4_byte_size;
    let mut to_ret = Vec::with_capacity(expected_num);
    for i in 0..bytes.len() / node4_byte_size {
        let i = i * node4_byte_size;
        let id = Id::from_bytes(&bytes[i..i + id_size]);
        let sockaddr = bytes_to_sockaddr(&bytes[i + id_size..i + node4_byte_size])?;
        let node = Node::<>::new(id, sockaddr);
        to_ret.push(node);
    }

    Ok(to_ret)
}

///  Node 数组转紧凑格式 (node_id + ip + port)
/// 
/// # Example
/// ```
/// # use yiilian_dht::routing_table::*;
/// # use yiilian_dht::util::*;
/// # use yiilian_dht::common::*;
/// 
/// use bytes::Bytes;
/// use std::net::{SocketAddr, ToSocketAddrs};
/// 
/// let data: Bytes = vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1, 127,0,0,1, 0,80].into();
/// let id = Id::from_bytes(&vec![0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1]);
/// let sock_addr: SocketAddr = "127.0.0.1:80".parse().unwrap();
/// let node = Node::new(id, sock_addr);
/// let rst = nodes4_to_bytes(&vec![node], 20);
/// assert_eq!(data, rst);
/// ```
pub fn nodes4_to_bytes(nodes: &[Node], id_size: usize) -> Vec<u8> {
    let node4_byte_size: usize = id_size + 6;
    let mut to_ret = Vec::with_capacity(node4_byte_size * nodes.len());
    for node in nodes {
        to_ret.append(&mut node.id.to_vec());
        to_ret.append(&mut sockaddr_to_bytes(&node.address));
    }
    to_ret
}

#[macro_export]
macro_rules! merge_socket_addr_bytes {
    {$addrs: expr} => {
        {
            let mut values = vec![];
            for item in $addrs {
                let mut addr_bytes = yiilian_core::util::sockaddr_to_bytes(&item);

                values.append(&mut addr_bytes);
            }

            let rst: bytes::Bytes = values.into();
            rst
        }
    };
}

/// 用于将 bytes::Bytes 类型的数据按照 len 分割成 Vec<Bytes>
#[macro_export]
macro_rules! split_bytes {
    {$self: ident, $len: expr} => {
        {
            use bytes::Bytes;

            let bytes = $self;
            if bytes.len() % $len == 0 {
                let mut rst: Vec<Bytes> = vec![];
                for i in 0..(bytes.len() / $len) {
                    let tmp = Bytes::copy_from_slice(&bytes[(i * $len)..((i+1) * $len)]);
                    rst.push(tmp.into());
                }
        
                Ok(rst)
            } else {
                Err(YiiLianError::FrameParse(anyhow!("split_bytes is error")))
            }
        }
    };
}

/// 用于将 Vec<Bytes> 类型的数据连接并转换成 bytes::Bytes
#[macro_export]
macro_rules! merge_node_bytes {
    {$nodes: expr, $len: expr} => {
        {
            let node_bytes: Vec<u8> = crate::common::util::nodes4_to_bytes($nodes, $len);
            let rst: bytes::Bytes = node_bytes.into();
            rst
        }
    };
}
