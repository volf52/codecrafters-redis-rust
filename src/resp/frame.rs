use std::{convert::TryFrom, fmt::Write};

use crate::resp::error::RespError;
use bytes::{Bytes, BytesMut};

use super::constants::{starting_byte, starting_char, NULL_STR, NULL_STR_BYTES};
use super::{buffer::RespBuffer, error::RespResult};

#[derive(Debug, PartialEq, Clone)]
pub enum RespFrame {
    Simple(String),
    Error(String),
    Integer(usize),
    Bulk(String),
    Null,
    Array(Vec<RespFrame>),
}

impl TryFrom<bytes::BytesMut> for RespFrame {
    type Error = RespError;

    fn try_from(value: bytes::BytesMut) -> Result<Self, <Self as TryFrom<bytes::BytesMut>>::Error> {
        let mut buff = RespBuffer::new(value);

        // let frame = RespFrame::parse_frame(&mut buff)?;
        let frame = RespFrame::parse_commands(&mut buff)?;

        Ok(frame)
    }
}

impl RespFrame {
    fn parse_commands(b: &mut RespBuffer) -> RespResult<Self> {
        let frame = match b.get_u8()? {
            starting_byte::ARRAY => Self::parse_array(b)?,
            c => {
                let e = format!("Invalid start for command: '{}' (should be '*')", c);

                RespFrame::Error(e)
            }
        };

        Ok(frame)
    }

    // For internal use only
    fn parse_frame(b: &mut RespBuffer) -> RespResult<Self> {
        let frame = match b.get_u8()? {
            starting_byte::SIMPLE_STRING => Self::parse_simple_string(b)?,
            starting_byte::ARRAY => Self::parse_array(b)?,
            starting_byte::BULK_STR => Self::parse_bulk_string(b)?,
            starting_byte::INT => Self::parse_integer(b)?,
            starting_byte::ERROR => Self::parse_error(b)?,
            c => {
                let e = format!("Invalid Start: {}", c as char);

                return Err(RespError::ParseError(e));
            }
        };

        Ok(frame)
    }

    fn parse_simple_string(b: &mut RespBuffer) -> RespResult<Self> {
        let t = String::from_utf8(b.get_line()?).unwrap();

        Ok(Self::Simple(t))
    }

    fn parse_integer(b: &mut RespBuffer) -> RespResult<Self> {
        let i = b.get_int()?;

        Ok(RespFrame::Integer(i))
    }

    fn parse_error(b: &mut RespBuffer) -> RespResult<Self> {
        let err_bytes = b.get_line()?;
        let err_str = String::from_utf8(err_bytes).unwrap();

        Ok(RespFrame::Error(err_str))
    }

    fn parse_array(b: &mut RespBuffer) -> RespResult<Self> {
        let total = b.get_int()?;

        let mut frame_vec = Vec::with_capacity(total);

        for _ in 0..total {
            let frame = Self::parse_frame(b)?;
            frame_vec.push(frame);
        }

        Ok(Self::Array(frame_vec))
    }

    fn parse_bulk_string(b: &mut RespBuffer) -> RespResult<Self> {
        let n_chars = b.get_int()?;
        let ss_bytes = b.get_line()?;
        let ss_bytes_len = ss_bytes.len();
        let ss = String::from_utf8(ss_bytes).unwrap();

        if n_chars != ss_bytes_len {
            return Err(RespError::InvalidCharsInBulkString(n_chars, ss));
        }

        Ok(Self::Bulk(ss))
    }

    pub fn write_to_buffer(&self, b: &mut BytesMut) {
        match self {
            Self::Error(e) => {
                b.write_char(starting_char::ERROR).unwrap();

                b.write_str(e).unwrap();

                write_endline(b);
            }
            Self::Simple(s) => {
                b.write_char(starting_char::SIMPLE_STRING).unwrap();

                b.write_str(s).unwrap();

                write_endline(b);
            }
            Self::Integer(i) => {
                b.write_char(starting_char::INT).unwrap();

                write_usize(b, *i);

                write_endline(b);
            }
            Self::Bulk(s) => {
                b.write_char(starting_char::BULK_STR).unwrap();

                write_usize(b, s.as_bytes().len());

                write_endline(b);

                b.write_str(s).unwrap();

                write_endline(b);
            }
            Self::Array(arr) => {
                b.write_char(starting_char::ARRAY).unwrap();

                write_usize(b, arr.len());
                write_endline(b);

                for frame in arr {
                    frame.write_to_buffer(b);
                }
            }
            Self::Null => {
                b.write_str(NULL_STR).unwrap();
            }
        }
    }

    pub fn to_bytes(&self) -> bytes::Bytes {
        if self.eq(&Self::Null) {
            return Bytes::from_static(NULL_STR_BYTES);
        }

        let mut b = BytesMut::new();

        self.write_to_buffer(&mut b);

        b.freeze()
    }

    pub fn process_commands(&self) -> Self {
        match self {
            Self::Array(arr) => {
                let v = Vec::new();
                let mut arr_iter = arr.iter();

                if let Some(RespFrame::Bulk(command)) = arr_iter.next() {
                    if command.eq("PING") {
                        return RespFrame::Simple("PONG".to_string());
                    }
                    if command.eq("ECHO") {
                        return match arr_iter.next() {
                            // warn: should check if is bulk string
                            Some(echo) => echo.clone(),
                            None => RespFrame::Error("Invalid Echo Command".to_string()),
                        };
                    }

                    return RespFrame::Error(format!("UNRECOGNIZED COMMAND: '{}'", command));
                }

                RespFrame::from(v)
            }
            _ => Self::Error("Only commands should be sent".to_string()),
        }
    }
}

impl From<Vec<RespFrame>> for RespFrame {
    fn from(arr: Vec<RespFrame>) -> Self {
        Self::Array(arr)
    }
}

#[inline]
fn write_endline(b: &mut BytesMut) {
    b.write_char('\r').unwrap();
    b.write_char('\n').unwrap();
}

#[inline]
fn write_usize(b: &mut BytesMut, i: usize) {
    b.write_str(&i.to_string()).unwrap();
}
