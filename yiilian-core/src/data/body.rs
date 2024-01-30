use bytes::{Buf, Bytes};

use super::{Request, Response};

pub trait Body: Clone {
    type Data: Buf;

    fn get_data(&mut self) -> Self::Data;

    fn len(&self) -> usize;
}

impl<B: Body> Body for Request<B> {
    type Data = B::Data;

    fn get_data(&mut self) -> Self::Data {
        self.body.get_data()
    }

    fn len(&self) -> usize {
        self.body.len()
    }
}

impl<B: Body> Body for Response<B> {
    type Data = B::Data;

    fn get_data(&mut self) -> Self::Data {
        self.body.get_data()
    }

    fn len(&self) -> usize {
        self.body.len()
    }
}


impl Body for String {
    type Data = Bytes;

    fn get_data(&mut self) -> Self::Data {
        let s = std::mem::take(&mut *self);
        s.into_bytes().into()
    }

    fn len(&self) -> usize {
        self.len()
    }
}

impl<T: Body + Default> Body for Option<T> 
{
    type Data = T::Data;

    fn get_data(&mut self) -> Self::Data {
        match self {
            Some(val) => val.get_data(),
            None => T::default().get_data(),
        }
    }

    fn len(&self) -> usize {
        match self {
            Some(val) => val.len(),
            None => 0,
        }
    }
}