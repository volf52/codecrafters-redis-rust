use bytes::Buf;

use super::error::{RespError, RespResult};

pub struct RespBuffer(bytes::BytesMut);

impl RespBuffer {
    pub fn new(b: bytes::BytesMut) -> Self {
        Self(b)
    }

    pub fn get_u8(&mut self) -> RespResult<u8> {
        let res = self.0.first().copied().ok_or(RespError::EndOfStream)?;

        self.0.advance(1);

        Ok(res)
    }

    pub fn get_int(&mut self) -> RespResult<usize> {
        let ss = String::from_utf8(self.get_line()?).unwrap();
        let u = ss
            .parse::<usize>()
            .map_err(|_| RespError::InvalidTotalForArray(ss))?;

        Ok(u)
    }

    pub fn get_line(&mut self) -> RespResult<Vec<u8>> {
        let mut iter = self.0.iter();
        let mut x = Vec::new();

        for v in iter.by_ref() {
            let c = *v;

            if c == b'\r' {
                break;
            }

            x.push(c);
        }

        if iter.next().eq(&Some(&b'\n')) {
            self.0.advance(x.len() + 2);

            Ok(x)
        } else {
            Err(RespError::InvalidEnd)
        }
    }
}
