#[derive(Debug)]
pub enum RespError {
    EndOfStream,
    InvalidEnd,
    InvalidCharsInBulkString(usize, String),
    InvalidTotalForArray(String),
    ParseError(String),
}

impl std::fmt::Display for RespError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EndOfStream => write!(f, "no more bytes left in the stream"),
            Self::ParseError(src) => write!(f, "Error parsing frame: {}", src),
            Self::InvalidEnd => write!(f, "No \n after \r"),
            Self::InvalidTotalForArray(total) => write!(f, "Invalid total for array: {}", total),
            Self::InvalidCharsInBulkString(n, s) => {
                write!(f, "Invalid number of chars ({}) in bulk string: {}", n, s)
            }
        }
    }
}

impl std::error::Error for RespError {}

pub type RespResult<T> = core::result::Result<T, RespError>;
