use yiilian_core::common::error::Error;

pub fn bytes_to_u32(bytes: &[u8]) -> Result<u32, Error> {
    let array: [u8; 4] = bytes.try_into().map_err(|_| {
        Error::new_frame(
            None,
            Some(format!("Can't convert slice to [u8; 4]: {:?}", bytes)),
        )
    })?;

    Ok(u32::from_be_bytes(array))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test() {
        let bytes = [0, 0, 0, 2, b'a', b'b'];
        assert_eq!(true,  bytes_to_u32(&bytes).is_err());

        let bytes = [0, 0, 0];
        assert_eq!(true,  bytes_to_u32(&bytes).is_err());

        let bytes = [0, 0, 0, 1];
        assert_eq!(true,  bytes_to_u32(&bytes).is_ok());
    }
}