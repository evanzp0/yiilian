use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::Path;
use std::str::FromStr;

use fnv::FnvHasher;

use crate::common::error::Error;

/// Macro for creating a [`HashMap`](std::collections::HashMap).
#[macro_export]
macro_rules! map {
    {$($k: expr => $v: expr),* $(,)?} => {
        std::collections::BTreeMap::from([$(($k, $v),)*])
    };
}

/// Macro for creating a Arc<Mutex> object.
#[macro_export]
macro_rules! arcmut {
    {$k: expr} => {
        Arc::new(Mutex::new($k))
    };
}

pub fn setup_log4rs_from_file<P: AsRef<Path>>(file_path: &P) {
    log4rs::init_file(file_path, Default::default()).unwrap();
}

/// convert string slice to int
///
/// # Examples
/// ```
/// # use yiilian_core::common::util::*;
///
/// assert_eq!(-12, atoi::<i32>("-12".as_bytes()).unwrap());
/// ```
pub fn atoi<T>(data: &[u8]) -> Result<T, Error> 
where
    T: std::str::FromStr,
    <T as FromStr>::Err: std::error::Error + Sync + Send,
{
    let s = String::from_utf8_lossy(data);
    let rst = s
        .parse::<T>()
        .map_err(|_| Error::new_frame(None, Some("atoi error".to_owned())))?;

    Ok(rst)
}

/// random_string generates a size-length bytes randomly.
///
/// # Example:
///
/// ```
/// # use yiilian_core::common::util::*;
///
/// assert_eq!(3, random_bytes(3).len());
/// ```
pub fn random_bytes(size: usize) -> Vec<u8> {
    let random_bytes: Vec<u8> = (0..size).map(|_| rand::random::<u8>()).collect();

    random_bytes.into()
}

/// 将 4 个大端字节数组转为 u32
pub fn be_bytes_to_u32(bytes: &[u8]) -> Result<u32, Error> {
    let array: [u8; 4] = bytes.try_into().map_err(|_| {
        Error::new_frame(
            None,
            Some(format!("Can't convert slice to [u8; 4]: {:?}", bytes)),
        )
    })?;

    Ok(u32::from_be_bytes(array))
}

/// 将 2 个小端字节数组转为 u16
pub fn be_bytes_to_u16(bytes: &[u8]) -> Result<u16, Error> {
    let array: [u8; 2] = bytes.try_into().map_err(|_| {
        Error::new_frame(
            None,
            Some(format!("Can't convert slice to [u8; 2]: {:?}", bytes)),
        )
    })?;

    Ok(u16::from_be_bytes(array))
}

/// 紧凑格式转 ip
///
/// # Example
/// ```
/// # use yiilian_core::common::util::*;
/// use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
///
/// let compacted_ip = vec![127,0,0,1];
/// let ip = bytes_to_ip(&compacted_ip).unwrap();
/// assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
/// ```
pub fn bytes_to_ip(bytes: &[u8]) -> Result<IpAddr, Error> {
    let bytes = bytes.as_ref();
    match bytes.len() {
        4 => {
            let ip = Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]);

            Ok(IpAddr::V4(ip))
        }

        16 => {
            let b0 = be_bytes_to_u16(&bytes[0..=1])?;
            let b1 = be_bytes_to_u16(&bytes[2..=3])?;
            let b2 = be_bytes_to_u16(&bytes[4..=5])?;
            let b3 = be_bytes_to_u16(&bytes[6..=7])?;
            let b4 = be_bytes_to_u16(&bytes[8..=9])?;
            let b5 = be_bytes_to_u16(&bytes[10..=11])?;
            let b6 = be_bytes_to_u16(&bytes[12..=13])?;
            let b7 = be_bytes_to_u16(&bytes[14..=15])?;

            let ip = Ipv6Addr::new(b0, b1, b2, b3, b4, b5, b6, b7);

            Ok(IpAddr::V6(ip))
        },

        _ => Err(Error::new_frame(
            None,
            Some("Wrong number of bytes for ip".to_owned()),
        )),
    }
}

/// IP 转紧凑格式
///
/// # Example
/// ```
/// # use yiilian_core::common::util::*;
/// use std::net::{SocketAddr, ToSocketAddrs};
///
/// let sockaddr: SocketAddr = "127.0.0.1:80".parse().unwrap();
/// let compacted_ip_port = sockaddr_to_bytes(&sockaddr);
/// assert_eq!(vec![127,0,0,1, 0,80], compacted_ip_port)
/// ```
pub fn ip_to_bytes(ip: &IpAddr) -> Vec<u8> {
    let mut to_ret = Vec::new();

    match ip {
        IpAddr::V4(v4) => {
            let ip_bytes = v4.octets();
            for item in ip_bytes {
                to_ret.push(item);
            }
        }

        IpAddr::V6(v6) => {
            let ip_bytes = v6.octets();
            for item in ip_bytes {
                to_ret.push(item);
            }
        }
    }

    to_ret
}

/// 紧凑格式转 SocketAddr (ip + port)
///
/// # Example
/// ```
/// # use yiilian_core::common::util::*;
/// use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
///
/// let compacted_ip_port = vec![127,0,0,1, 0,80];
/// let sockaddr = bytes_to_sockaddr(&compacted_ip_port).unwrap();
/// assert_eq!(sockaddr.ip(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
/// assert_eq!(sockaddr.port(), 80);
/// ```
pub fn bytes_to_sockaddr(bytes: &[u8]) -> Result<SocketAddr, Error> {
    let bytes = bytes.as_ref();
    match bytes.len() {
        6 => {
            let ip = Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]);

            let port_bytes_as_array: [u8; 2] =
                bytes[4..6]
                    .try_into()
                    .map_err(|e: std::array::TryFromSliceError| {
                        Error::new_frame(Some(Box::new(e)), None)
                    })?;

            let port: u16 = u16::from_be_bytes(port_bytes_as_array);

            Ok(SocketAddr::new(IpAddr::V4(ip), port))
        }

        18 => Err(Error::new_frame(
            None,
            Some("IPv6 is not yet implemented".to_owned()),
        )),

        _ => Err(Error::new_frame(
            None,
            Some("Wrong number of bytes for sockaddr".to_owned()),
        )),
    }
}

/// SocketAddr 转紧凑格式 (ip + port)
///
/// # Example
/// ```
/// # use yiilian_core::common::util::*;
/// use std::net::{SocketAddr, ToSocketAddrs};
///
/// let sockaddr: SocketAddr = "127.0.0.1:80".parse().unwrap();
/// let compacted_ip_port = sockaddr_to_bytes(&sockaddr);
/// assert_eq!(vec![127,0,0,1, 0,80], compacted_ip_port)
/// ```
pub fn sockaddr_to_bytes(sockaddr: &SocketAddr) -> Vec<u8> {
    let mut to_ret = Vec::new();

    match sockaddr {
        SocketAddr::V4(v4) => {
            let ip_bytes = v4.ip().octets();
            for item in ip_bytes {
                to_ret.push(item);
            }
        }

        SocketAddr::V6(v6) => {
            let ip_bytes = v6.ip().octets();
            for item in ip_bytes {
                to_ret.push(item);
            }
        }
    }

    let port_bytes = sockaddr.port().to_be_bytes();
    to_ret.push(port_bytes[0]);
    to_ret.push(port_bytes[1]);

    to_ret
}

/// get a u64 hash code
pub fn hash_it<T: Hash>(name: T) -> u64 {
    let mut hasher = FnvHasher::default();
    name.hash(&mut hasher);
    hasher.finish()
}

pub fn binary_insert<T: Ord>(containers: &mut Vec<T>, item: T, dup: bool) {
    match containers.binary_search(&item) {
        Ok(pos) => {
            if dup {
                containers.insert(pos, item)
            }
        }
        Err(pos) => containers.insert(pos, item),
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;
    use super::*;

    #[test]
    fn test_hashmap_macro() {
        let map: BTreeMap<String, String> = map! {
            "en".into() => "Goodbye".into(),
            "de".into() => "Auf Wiedersehen".into(),
        };

        let mut map1: BTreeMap<String, String> = BTreeMap::new();
        map1.insert("en".into(), "Goodbye".into());
        map1.insert("de".into(), "Auf Wiedersehen".into());

        assert_eq!(map1, map);
    }

    #[test]
    fn test_be_bytes_to_u32() {
        let bytes = [0, 0, 0, 2, b'a', b'b'];
        assert_eq!(true,  be_bytes_to_u32(&bytes).is_err());

        let bytes = [0, 0, 0];
        assert_eq!(true,  be_bytes_to_u32(&bytes).is_err());

        let bytes = [0, 0, 0, 1];
        assert_eq!(true,  be_bytes_to_u32(&bytes).is_ok());
    }

    #[test]
    fn test_be_bytes_to_u16() {
        let bytes = [255, 0];
        assert_eq!(65280,  be_bytes_to_u16(&bytes).unwrap());
    }

    #[test]
    fn test_atoi() {
        let num: u64 = atoi(b"000012").unwrap();
        assert_eq!(12, num);
    }

    #[test]
    fn test_binary_insert() {
        let mut array = vec![];

        binary_insert(&mut array, 2, false);
        binary_insert(&mut array, 2, false);
        binary_insert(&mut array, 0, false);
        binary_insert(&mut array, 5, false);

        assert_eq!(0, *array.get(0).unwrap());
        assert_eq!(2, *array.get(1).unwrap());
        assert_eq!(5, *array.get(2).unwrap());

        binary_insert(&mut array, 2, true);
        assert_eq!(2, *array.get(2).unwrap());
    }
}
