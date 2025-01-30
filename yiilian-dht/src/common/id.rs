use std::{collections::HashSet, net::IpAddr};

use bytes::Bytes;
use rand::{thread_rng, Rng};
use yiilian_core::common::{error::Error, expect_log::ExpectLog};

use super::util::CASTAGNOLI;

pub const ID_SIZE: usize = 20;

/// Represents the id of a dht Node or a BitTorrent info-hash.
/// Basically, it's a 20-byte identifier.
#[derive(Eq, PartialEq, Hash, Clone, Copy)]
pub struct Id {
    bytes: [u8; ID_SIZE],
}

impl TryFrom<&[u8]> for Id {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() == ID_SIZE {
            let mut tmp: [u8; ID_SIZE] = [0; ID_SIZE];
            tmp[..ID_SIZE].clone_from_slice(&value[..ID_SIZE]);
    
            Ok( Id { bytes: tmp } )
        } else {
            Err(Error::new_id(None, Some(format!("&[u8] is too short to convert id, {:?}", value))))
        }
        
    }
}

impl TryFrom<&str> for Id {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.as_bytes().try_into()
    }
}

impl TryFrom<Bytes> for Id {
    type Error = Error;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        value.as_ref().try_into()
    }
}

impl Id {
    pub fn new(bytes: [u8; ID_SIZE]) -> Self {
        Id {
            bytes
        }
    }

    pub fn get_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(&self.bytes)
    }

    /// Create a new Id from some bytes. Returns Err if `bytes` is not of length ID_SIZE
    pub fn from_bytes(bytes: &[u8]) -> Result<Id, Error> {
        bytes.try_into()
    }

    /// Generates a random Id for a mainline DHT node with the provided IP address.
    /// The generated Id will be valid with respect to [BEP0042](http://bittorrent.org/beps/bep_0042.html).
    /// 根据 ip 生成可校验 node_id
    pub fn from_ip(ip: &IpAddr) -> Id {
        let mut rng = thread_rng();
        let r: u8 = rng.gen();

        let magic_prefix = IdPrefixMagic::from_ip(ip, r);

        let mut bytes = [0 as u8; ID_SIZE];

        bytes[0] = magic_prefix.prefix[0];
        bytes[1] = magic_prefix.prefix[1];
        // 第三个字节取 magic_prefix.prefix[2] 的前 5 位，其余 3 位随机生成。例如：1010_0000 -> 1010_0xxx (xxx 为随机生成的)
        bytes[2] = (magic_prefix.prefix[2] & 0xf8) | (rng.gen::<u8>() & 0x7);

        // 生成 35 random bit
        for item in bytes.iter_mut().take(ID_SIZE - 1).skip(3) {
            *item = rng.gen();
        }

        // 最后一个字节为 r
        // todo！这个 r 随机数，应该和 crc32c 的最后一个字节相同，否则校验会失败。此处暂时没有实现这一点。
        bytes[ID_SIZE - 1] = r; 

        Id { bytes }
    }

    /// Generates a completely random Id. The returned Id is *not* guaranteed to be
    /// valid with respect to [BEP0042](http://bittorrent.org/beps/bep_0042.html).
    pub fn from_random<T: rand::RngCore>(rng: &mut T) -> Id {
        let mut bytes = [0 as u8; ID_SIZE];
        rng.fill_bytes(&mut bytes);

        Id { bytes }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Copies the byes that make up the Id and returns them in a Vec
    pub fn to_vec(&self) -> Vec<u8> {
        self.bytes.to_vec()
    }

    /// Evaluates the Id and decides if it's a valid Id for a DHT node with the
    /// provided IP address (based on [BEP0042](http://bittorrent.org/beps/bep_0042.html)).
    /// Note: the current implementation does not handle non-globally-routable address space
    /// properly. It will likely return false for non-routable IPv4 address space (against the spec).
    pub fn is_valid_for_ip(&self, ip: &IpAddr, white_list: &HashSet<IpAddr>) -> bool {
        // TODO return true if ip is not globally routable, for example: localhost test
        if ip.is_loopback() || white_list.contains(ip) {
            return true;
        }
        let len = self.len();
        let expected = IdPrefixMagic::from_ip(ip, self.bytes[len - 1]);

        match IdPrefixMagic::from_id(self) {
            Ok(actual) => expected == actual,
            Err(_) => false,
        }
    }

    /// Returns the number of bits of prefix that the two ids have in common.
    /// 返回 self.bytes 和 other.bytes 的最长公共位的长度, CPL
    ///
    /// Consider two Ids with binary values `10100000` and `10100100`. This function
    /// would return `5` because the Ids share the common 5-bit prefix `10100`.
    pub fn matching_prefix_bits(&self, other: &Self) -> usize {
        let xored = self.xor(other);
        let mut to_ret: usize = 0;

        for i in 0..ID_SIZE {
            let leading_zeros: usize = xored.bytes[i]
                .leading_zeros() // 返回前置的 0 bit 个数
                .try_into()
                .expect("this should never fail");
            to_ret += leading_zeros;

            // 前置的 0 bit 个数不满 8位（不满一个字节），跳出循环，否则，继续循环下一个字节
            if leading_zeros < 8 {
                break;
            }
        }

        to_ret
    }

    /// Creates an Id from a hex string.
    ///
    /// For example: `let id = Id::from_hex("88ffb73943354a00dc2dadd14c54d28020a513c8").unwrap();`
    pub fn from_hex(h: &str) -> Result<Id, Error> {
        let bytes = hex::decode(h).map_err(|hex_err| Error::new_id(Some(hex_err.into()), None))?;

        Id::from_bytes(&bytes)
    }

    /// Computes the exclusive or (XOR) of this Id with another. The BitTorrent DHT
    /// uses XOR as its distance metric.
    ///
    /// Example: `let distance_between_nodes = id.xor(other_id);`
    pub fn xor(&self, other: &Id) -> Id {
        let mut bytes = vec![0; ID_SIZE];
        for (i, item) in bytes.iter_mut().enumerate() {
            *item = self.bytes[i] ^ other.bytes[i];
        }

        Id::from_bytes(&bytes).expect_error("Id::from_bytes() error")
    }

    /// Makes a new id that's similar to this one.
    /// `identical_bytes` specifies how many bytes of the resulting Id should be the same as `this`.
    /// `identical_bytes` must be in the range (0, [ID_SIZE](crate::common::ID_SIZE)) otherwise Err
    /// is returned.
    ///
    /// 生成和本 ID 接近的新 ID
    /// identical_bytes ： 指明了新生成的 ID 有多少字节和本 ID 相同
    pub fn make_mutant(&self, identical_bytes: usize) -> Result<Id, Error> {
        let len = self.len();

        if identical_bytes == 0 || identical_bytes >= len {
            return Err(Error::new_general(
                "identical_bytes must be in range (0, ID_SIZE)",
            ));
        }
        let mut mutant = Id::from_random(&mut thread_rng()); // 生成一个随机 ID
        for i in 0..identical_bytes {
            mutant.bytes[i] = self.bytes[i]; // 生成的新 ID 前 identical_bytes 字节使用和本 ID 相同的字节
        }
        Ok(mutant)
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(&self.bytes))
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(&self.bytes))
    }
}

impl PartialOrd for Id {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        for i in 0..self.len() {
            match self.bytes[i].cmp(&other.bytes[i]) {
                std::cmp::Ordering::Less => return Some(std::cmp::Ordering::Less),
                std::cmp::Ordering::Greater => return Some(std::cmp::Ordering::Greater),
                _ => continue,
            };
        }

        Some(std::cmp::Ordering::Equal)
    }
}

#[derive(Debug)]
struct IdPrefixMagic {
    prefix: [u8; 3],
    suffix: u8,
}

impl IdPrefixMagic {
    // Populates an IdPrefixMagic from the bytes of an id.
    // This isn't a way to generate a valid IdPrefixMagic from a random id.
    // For that, use Id::from_ip
    fn from_id(id: &Id) -> Result<IdPrefixMagic, Error> {
        if id.len() < 4 {
            return Err(Error::new_id(None, Some("Wrong number of id's bytes".to_owned())));
        }

        let rst = IdPrefixMagic {
            prefix: id.bytes[..3]
                .try_into()
                .expect("Failed to grab first three bytes of id"),
            suffix: id.bytes[id.len() - 1],
        };

        Ok(rst)
    }

    /// 根据 ip 和 seed_r 随机数种子，生成 IdPrefixMagic （该对象用于最终生成 validated node id）
    fn from_ip(ip: &IpAddr, seed_r: u8) -> IdPrefixMagic {
        match ip {
            IpAddr::V4(ipv4) => {
                let r32: u32 = seed_r.into();
                let magic: u32 = 0x030f3fff;
                let ip_int: u32 = u32::from_be_bytes(ipv4.octets()); //todo! 应该 (ip_int & magic) | (r32 << 29)后取大端序，在计算 hash ，这里顺序错了
                let nonsense: u32 = (ip_int & magic) | (r32 << 29);
                let crc: u32 = CASTAGNOLI.checksum(&nonsense.to_be_bytes());
                IdPrefixMagic {
                    prefix: crc.to_be_bytes()[..3]
                        .try_into()
                        .expect("Failed to convert bytes 0-2 of the crc into a 3-byte array"),
                    suffix: seed_r,
                }
            }
            IpAddr::V6(ipv6) => {
                let r64: u64 = seed_r.into();
                let magic: u64 = 0x0103070f1f3f7fff;
                let ip_int: u64 = u64::from_be_bytes(
                    ipv6.octets()[8..]
                        .try_into()
                        .expect("Failed to get IPv6 bytes"),
                );
                let nonsense: u64 = ip_int & magic | (r64 << 61);
                let crc: u32 = CASTAGNOLI.checksum(&nonsense.to_be_bytes());
                IdPrefixMagic {
                    prefix: crc.to_be_bytes()[..2].try_into().expect("Failed to poop"),
                    suffix: seed_r,
                }
            }
        }
    }
}

impl PartialEq for IdPrefixMagic {
    fn eq(&self, other: &Self) -> bool {
        self.prefix[0] == other.prefix[0]
            && self.prefix[1] == other.prefix[1]
            && self.prefix[2] & 0xf8 == other.prefix[2] & 0xf8
            && self.suffix == other.suffix
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_from_ip_v4() {
        assert_eq!(
            IdPrefixMagic::from_ip(&IpAddr::V4(Ipv4Addr::new(124, 31, 75, 21)), 1),
            IdPrefixMagic {
                prefix: [0x5f, 0xbf, 0xbf],
                suffix: 1
            }
        );
        assert_eq!(
            IdPrefixMagic::from_ip(&IpAddr::V4(Ipv4Addr::new(21, 75, 31, 124)), 86),
            IdPrefixMagic {
                prefix: [0x5a, 0x3c, 0xe9],
                suffix: 0x56
            }
        );

        assert_eq!(
            IdPrefixMagic::from_ip(&IpAddr::V4(Ipv4Addr::new(65, 23, 51, 170)), 22),
            IdPrefixMagic {
                prefix: [0xa5, 0xd4, 0x32],
                suffix: 0x16
            }
        );

        assert_eq!(
            IdPrefixMagic::from_ip(&IpAddr::V4(Ipv4Addr::new(84, 124, 73, 14)), 65),
            IdPrefixMagic {
                prefix: [0x1b, 0x03, 0x21],
                suffix: 0x41
            }
        );

        assert_eq!(
            IdPrefixMagic::from_ip(&IpAddr::V4(Ipv4Addr::new(43, 213, 53, 83)), 90),
            IdPrefixMagic {
                prefix: [0xe5, 0x6f, 0x6c],
                suffix: 0x5a
            }
        );
    }

    #[test]
    fn test_generate_valid_id() {
        let ip = IpAddr::V4(Ipv4Addr::new(124, 31, 75, 21));
        let id = Id::from_ip(&ip);
        assert!(id.is_valid_for_ip(&ip, &HashSet::new()));
    }

    #[test]
    fn test_id_xor() {
        let h1 = Id::from_hex("0000000000000000000000000000000000000001").unwrap();
        let h2 = Id::from_hex("0000000000000000000000000000000000000000").unwrap();
        let h3 = h1.xor(&h2);
        assert_eq!(h3, h1);

        let h1 = Id::from_hex("0000000000000000000000000000000000000001").unwrap();
        let h2 = Id::from_hex("0000000000000000000000000000000000000001").unwrap();
        let h3 = h1.xor(&h2);
        assert_eq!(
            h3,
            Id::from_hex("0000000000000000000000000000000000000000").unwrap()
        );

        let h1 = Id::from_hex("1010101010101010101010101010101010101010").unwrap();
        let h2 = Id::from_hex("0101010101010101010101010101010101010101").unwrap();
        let h3 = h1.xor(&h2);
        assert_eq!(
            h3,
            Id::from_hex("1111111111111111111111111111111111111111").unwrap()
        );

        let h1 = Id::from_hex("fefefefefefefefefefefefefefefefefefefefe").unwrap();
        let h2 = Id::from_hex("0505050505050505050505050505050505050505").unwrap();
        let h3 = h1.xor(&h2);
        assert_eq!(
            h3,
            Id::from_hex("fbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfb").unwrap()
        );
    }

    #[test]
    fn test_id_ordering() {
        let h1 = Id::from_hex("0000000000000000000000000000000000000001").unwrap();
        let h2 = Id::from_hex("0000000000000000000000000000000000000000").unwrap();
        assert!(h1 > h2);
        assert!(h2 < h1);
        assert_ne!(h1, h2);

        let h1 = Id::from_hex("00000000000000000000f0000000000000000000").unwrap();
        let h2 = Id::from_hex("000000000000000000000fffffffffffffffffff").unwrap();
        assert!(h1 > h2);
        assert!(h2 < h1);
        assert_ne!(h1, h2);

        let h1 = Id::from_hex("1000000000000000000000000000000000000000").unwrap();
        let h2 = Id::from_hex("0fffffffffffffffffffffffffffffffffffffff").unwrap();
        assert!(h1 > h2);
        assert!(h2 < h1);
        assert_ne!(h1, h2);

        let h1 = Id::from_hex("1010101010101010101010101010101010101010").unwrap();
        let h2 = Id::from_hex("1010101010101010101010101010101010101010").unwrap();
        assert!(h1 <= h2);
        assert!(h2 <= h1);
        assert_eq!(h1, h2);

        let h1 = Id::from_hex("0000000000000000000000000000000000000010").unwrap();
        let h2 = Id::from_hex("0000000000000000000000000000000000000001").unwrap();
        assert!(h1 > h2);
        assert!(h2 < h1);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_matching_prefix_bits() {
        let h1 = Id::from_hex("0000000000000000000000000000000000000000").unwrap();
        let h2 = Id::from_hex("0000000000000000000000000000000000000000").unwrap();
        assert_eq!(h1.matching_prefix_bits(&h2), 160);

        let h1 = Id::from_hex("0000000000000000000000000000000000000000").unwrap();
        let h2 = Id::from_hex("f000000000000000000000000000000000000000").unwrap();
        assert_eq!(h1.matching_prefix_bits(&h2), 0);

        let h1 = Id::from_hex("0000000000000000000000000000000000000000").unwrap();
        let h2 = Id::from_hex("1000000000000000000000000000000000000000").unwrap();
        assert_eq!(h1.matching_prefix_bits(&h2), 3);
    }
}
