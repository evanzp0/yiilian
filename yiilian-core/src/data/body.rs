use bytes::{Buf, Bytes};

use super::{Request, Response};

pub trait Body {
    type Data: Buf;

    fn data(&mut self) -> Self::Data;

    fn len(&self) -> usize;
}

impl<B: Body> Body for Request<B> {
    type Data = B::Data;

    fn data(&mut self) -> Self::Data {
        self.body.data()
    }

    fn len(&self) -> usize {
        self.body.len()
    }
}

impl<B: Body> Body for Response<B> {
    type Data = B::Data;

    fn data(&mut self) -> Self::Data {
        self.body.data()
    }

    fn len(&self) -> usize {
        self.body.len()
    }
}


impl Body for String {
    type Data = Bytes;

    fn data(&mut self) -> Self::Data {
        let s = std::mem::take(&mut *self);
        s.into_bytes().into()
    }

    fn len(&self) -> usize {
        self.len()
    }
}

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

// unsafe impl Send for RawBody {}