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
