use std::{convert::TryFrom, fmt::Write};

use crate::resp::error::RespError;
use bytes::{Bytes, BytesMut};
use tokio::sync::{broadcast, mpsc};

use super::constants::{starting_byte, starting_char, NULL_STR, NULL_STR_BYTES};
use super::{buffer::RespBuffer, error::RespResult};
use crate::store::{StoreCommand, StoreResponse};

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

    pub async fn process_commands(
        &self,
        store_sender: &mut mpsc::Sender<StoreCommand>,
        store_receiver: &mut broadcast::Receiver<StoreResponse>,
    ) -> Self {
        match self {
            Self::Array(arr) => {
                let v = Vec::new();
                let mut arr_iter = arr.iter();

                if let Some(Self::Bulk(command)) = arr_iter.next() {
                    let cmd_lower = command.to_ascii_lowercase();

                    // process ping command
                    if cmd_lower.eq("ping") {
                        return Self::Simple("PONG".to_string());
                    }

                    // process echo command
                    if cmd_lower.eq("echo") {
                        return match arr_iter.next() {
                            // warn: should check if is bulk string
                            Some(echo) => echo.clone(),
                            None => Self::Error("Invalid Echo Command".to_string()),
                        };
                    }

                    // process get command
                    if cmd_lower.eq("get") {
                        return match arr_iter.next() {
                            Some(Self::Bulk(key)) => {
                                // check if present in store
                                let get_cmd = StoreCommand::get_value(key.clone());
                                let _res = store_sender.send(get_cmd).await;

                                match store_receiver.recv().await {
                                    Ok(StoreResponse::Value(v)) => Self::Bulk(v),
                                    Ok(StoreResponse::Nil) => Self::Null,
                                    Err(e) => Self::Error(e.to_string()),
                                    _ => unreachable!(),
                                }
                            }
                            _ => Self::Error(
                                "Invalid GET command: must send bulk string after this".to_string(),
                            ),
                        };
                    }

                    // process set command
                    if cmd_lower.eq("set") {
                        let key = arr_iter.next();
                        let val = arr_iter.next();
                        let PX = "PX".to_string();

                        let expires_in = match arr_iter.next() {
                            None => Ok(None),
                            Some(RespFrame::Bulk(PX)) =>
                                match arr_iter.next() {
                                    Some(RespFrame::Bulk(i)) => match i.parse::<u64>() {
                                        Ok(v) => Ok(Some(v)),
                                        Err(_) => Err(format!("Error parsing PX value: {}", i))
                                    },
                                    _ => Err("must supply expires_in value after PX".to_string()),
                                }
                            ,
                            _ => Err("Malformed command: Only PX can be optionally sent after SET <key> <value>".to_string())
                        };

                        return match (key, val, expires_in) {
                            (Some(Self::Bulk(k)), Some(Self::Bulk(v)), Ok(exp)) => {
                                let exp_duration = exp.map(std::time::Duration::from_millis);
                                let set_cmd =
                                    StoreCommand::set_value(k.clone(), v.clone(), exp_duration);

                                let _res = store_sender.send(set_cmd).await;

                                match store_receiver.recv().await {
                                    Ok(StoreResponse::Ok) => Self::Simple("OK".to_string()),
                                    Err(e) => Self::Error(e.to_string()),
                                    Ok(_) => unreachable!(),
                                }
                            }
                            (_, _, Err(e)) => Self::Error(e),
                            _ => Self::Error(
                                "Invalid SET command: must send two bulk strings afterwards"
                                    .to_string(),
                            ),
                        };
                    }

                    return Self::Error(format!("UNRECOGNIZED COMMAND: '{}'", command));
                }

                Self::from(v)
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
