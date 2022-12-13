pub(crate) const NULL_STR: &str = "$-1\r\n";
pub(crate) const NULL_STR_BYTES: &[u8] = NULL_STR.as_bytes();

pub(crate) mod starting_char {
    pub const SIMPLE_STRING: char = '+';
    pub const ERROR: char = '-';
    pub const BULK_STR: char = '$';
    pub const ARRAY: char = '*';
    pub const INT: char = ':';
}

pub(crate) mod starting_byte {
    use super::starting_char;

    pub const SIMPLE_STRING: u8 = starting_char::SIMPLE_STRING as u8;
    pub const ERROR: u8 = starting_char::ERROR as u8;
    pub const BULK_STR: u8 = starting_char::BULK_STR as u8;
    pub const ARRAY: u8 = starting_char::ARRAY as u8;
    pub const INT: u8 = starting_char::INT as u8;
}
