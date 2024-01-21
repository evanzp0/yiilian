use bytes::Bytes;
use yiilian_core::data::Body;

#[derive(Debug)]
pub struct RawBody {
    data: Bytes,
}

impl RawBody {
    pub fn new(data: Bytes) -> Self {
        Self { data }
    }

    pub fn from_str(data: &str) -> Self {
        let data = Bytes::copy_from_slice(data.as_bytes());
        Self {
            data
        }
    }
}

impl Default for RawBody {
    fn default() -> Self {
        Self { data: Default::default() }
    }
}

impl Body for RawBody {
    type Data = Bytes;

    fn data(&mut self) -> Self::Data {
        let s = std::mem::take(&mut *self);
        s.data
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}