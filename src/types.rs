use bytes::Bytes;

/// These types are used by state and ops to actually perform useful work.
pub type Value = Bytes;
/// Key is the standard type to index our structures
pub type Key = Bytes;
/// Count is used for commands that count.
pub type Count = i64;
/// Index is used to represent indices in structures.
pub type Index = i64;
/// Score is used in sorted sets
pub type Score = i64;
/// Timeout unit
pub type UTimeout = i64;
/// Bool type
pub type RedisBool = i64;

/// DumpTimeoutUnitpe alias.
// pub type DumpFile = Arc<Mutex<File>>;

/// RedisValueRef is the canonical type for values flowing
/// through the system. Inputs are converted into RedisValues,
/// and outputs are converted into RedisValues.
#[derive(PartialEq, Clone)]
pub enum RedisValueRef {
    String(Bytes),
    Error(Bytes),
    ErrorMsg(Vec<u8>),
    Int(i64),
    Array(Vec<RedisValueRef>),
    NullArray,
    NullBulkString,
}

impl std::fmt::Debug for RedisValueRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedisValueRef::String(s) => {
                write!(f, "RedisValueRef::String({:?})", String::from_utf8_lossy(s))
            }
            RedisValueRef::Error(s) => {
                write!(f, "RedisValueRef::Error({:?})", String::from_utf8_lossy(s))
            }
            RedisValueRef::ErrorMsg(s) => write!(f, "RedisValueRef::ErrorMsg({:?})", s),

            RedisValueRef::Int(i) => write!(f, "RedisValueRef::Int({:?})", i),
            RedisValueRef::NullBulkString => write!(f, "RedisValueRef::NullBulkString"),
            RedisValueRef::NullArray => write!(f, "RedisValueRef::NullArray"),
            RedisValueRef::Array(arr) => {
                write!(f, "RedisValueRef::Array(")?;
                for item in arr {
                    write!(f, "{:?}", item)?;
                    write!(f, ",")?;
                }
                write!(f, ")")?;
                Ok(())
            }
        }
    }
}

/// Special constants in the RESP protocol.
pub const NULL_BULK_STRING: &str = "$-1\r\n";
pub const NULL_ARRAY: &str = "*-1\r\n";
pub const EMPTY_ARRAY: &str = "*0\r\n";

/// Convenience type for returns value. Maps directly to RedisValues.
#[derive(Debug, PartialEq, Clone)]
pub enum ReturnValue {
    Ok,
    StringRes(Value),
    Error(&'static [u8]),
    MultiStringRes(Vec<Value>),
    Array(Vec<ReturnValue>),
    IntRes(i64),
    Nil,
}

/// Convenience trait to convert Count to ReturnValue.
impl From<Count> for ReturnValue {
    fn from(int: Count) -> ReturnValue {
        ReturnValue::IntRes(int)
    }
}

/// Convenience trait to convert ReturnValues to ReturnValue.
impl From<Vec<Value>> for ReturnValue {
    fn from(vals: Vec<Value>) -> ReturnValue {
        ReturnValue::Array(vals.into_iter().map(ReturnValue::StringRes).collect())
    }
}

/// Convenience trait to convert ReturnValues to ReturnValue.
impl From<Vec<String>> for ReturnValue {
    fn from(strings: Vec<String>) -> ReturnValue {
        let strings_to_bytes: Vec<Bytes> = strings
            .into_iter()
            .map(|s| s.as_bytes().to_vec().into())
            .collect();
        ReturnValue::MultiStringRes(strings_to_bytes)
    }
}

/// Convenience method to determine an error. Used in testing.
impl ReturnValue {
    pub fn is_error(&self) -> bool {
        if let ReturnValue::Error(_) = *self {
            return true;
        }
        false
    }
}
