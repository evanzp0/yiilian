use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use fnv::FnvHasher;

use crate::error::Error;

/// Macro for creating a [`HashMap`](std::collections::HashMap).
#[macro_export]
macro_rules! hashmap {
    {$($k: expr => $v: expr),* $(,)?} => {
        std::collections::HashMap::from([$(($k, $v),)*])
    };
}

/// Macro for creating a Arc<Mutex> object.
#[macro_export]
macro_rules! arcmut {
    {$k: expr} => {
        Arc::new(Mutex::new($k))
    };
}

/// 用于将 Vec<Bytes> 类型的数据连接并转换成 bytes::Bytes
#[macro_export]
macro_rules! extract_frame_map {
    {$map: ident, $field: expr, $frame: ident} => {
        {
            use crate::*;
            if let Some(value) = $map.get($field.as_bytes()) {
                // Ok(value.as_bstr().expect(&format!("err: {:?}", value)).clone())
                Ok(value.as_bstr().expect(&format!("err: {:?}", value)).clone())
            } else {
                Error::new_frame(
                    Some(Box::new(e)),
                    Some(format!("Reply frame is error, frame: {}",$frame.to_string()))
                )
            }
        }
    };
}

/// convert string slice to int
///
/// # Examples
/// ```
/// # use yiilian_core::util::*;
///
/// assert_eq!(-12, atoi("-12".as_bytes()).unwrap());
/// ```
pub fn atoi(data: &[u8]) -> Result<i32, Error> {
    let s = String::from_utf8_lossy(data);
    let rst = s
        .parse::<i32>()
        .map_err(|e| Error::new_frame(Some(Box::new(e)), Some("atoi error".to_owned())))?;

    Ok(rst)
}

/// random_string generates a size-length bytes randomly.
///
/// # Example:
///
/// ```
/// # use yiilian_core::util::*;
///
/// assert_eq!(3, random_bytes(3).len());
/// ```
pub fn random_bytes(size: usize) -> Vec<u8> {
    let random_bytes: Vec<u8> = (0..size).map(|_| rand::random::<u8>()).collect();

    random_bytes.into()
}

/// 紧凑格式转 SocketAddr (ip + port)
///
/// # Example
/// ```
/// # use yiilian_core::util::*;
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
/// # use yiilian_core::util::*;
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

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    #[test]
    fn test_hashmap_macro() {
        let map: HashMap<String, String> = hashmap! {
            "en".into() => "Goodbye".into(),
            "de".into() => "Auf Wiedersehen".into(),
        };

        let mut map1: HashMap<String, String> = HashMap::new();
        map1.insert("en".into(), "Goodbye".into());
        map1.insert("de".into(), "Auf Wiedersehen".into());

        assert_eq!(map1, map);
    }
}
