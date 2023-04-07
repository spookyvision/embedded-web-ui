use serde::{Deserialize, Serialize};

#[cfg(all(feature = "json", not(feature = "postcard")))]
pub(crate) mod imp {
    use super::*;
    pub fn encode<T>(value: &T) -> serde_json::Result<Vec<u8>>
    where
        T: Serialize + ?Sized,
    {
        let mut res = serde_json::to_vec(value)?;
        res.push(0);
        Ok(res)
    }

    pub fn decode<'a, T>(s: &'a mut [u8]) -> serde_json::Result<T>
    where
        T: Deserialize<'a>,
    {
        serde_json::from_slice(&s[0..s.len() - 1])
    }
}

#[cfg(all(feature = "postcard", not(feature = "json")))]
pub(crate) mod imp {
    use super::*;

    pub fn encode<T>(value: &T) -> postcard::Result<Vec<u8>>
    where
        T: Serialize + ?Sized,
    {
        postcard::to_allocvec_cobs(value)
    }

    pub fn decode<'a, T>(s: &'a mut [u8]) -> postcard::Result<T>
    where
        T: Deserialize<'a>,
    {
        postcard::from_bytes_cobs(s)
    }
}

pub use imp::*;
