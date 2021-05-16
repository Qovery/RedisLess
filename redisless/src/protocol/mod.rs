#[cfg(test)]
mod tests;

pub mod error;
pub mod parser;

use error::RedisError;

pub type Result<'a> = std::result::Result<(Resp<'a>, &'a [u8]), RedisError>;

const NIL_VALUE_SIZE: usize = 4;
const CR: u8 = b'\r';
const LF: u8 = b'\n';

pub const OK: &[u8; 5] = b"+OK\r\n";
pub const PONG: &[u8; 7] = b"+PONG\r\n";
pub const EMPTY_LIST: &[u8; 6] = b"$0\r\n\r\n";
pub const NIL: &[u8; 5] = b"$-1\r\n";

#[derive(Debug, Eq, PartialEq)]
pub enum Resp<'a> {
    String(&'a [u8]),
    Error(&'a [u8]),
    Integer(&'a [u8]),
    BulkString(&'a [u8]),
    Array(Vec<Resp<'a>>),
    Nil,
}
