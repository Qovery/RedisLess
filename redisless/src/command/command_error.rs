use std::{
    fmt::{Display, Formatter},
    num::ParseIntError,
    str::Utf8Error,
};

use crate::protocol::error::RedisError;
use crate::storage::models::expiry::TimeOverflow;

#[derive(Debug)]
pub enum RedisCommandError {
    // Wrong number of arguments, holds command
    ArgNumber,
    // Overflow when setting the expiry timestamp
    TimeOverflow(TimeOverflow),
    // Could not convert bytes to UTF8
    BadString(Utf8Error),
    // Could not parse string for a u64
    IntParse(ParseIntError),
    // Command is not supported by Redisless
    NotSupported(String),
    ProtocolParse(RedisError),
    InvalidCommand,
    CommandNotFound,
    // Wrong type operation against a key
    WrongTypeOperation,
}

impl RedisCommandError {
    pub fn to_vec(self) -> Vec<u8> {
        format!("-{}\r\n", self).as_bytes().to_vec()
    }
}

impl Display for RedisCommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArgNumber => {
                write!(f, "wrong number of arguments for command")
            }
            Self::TimeOverflow(e) => write!(f, "{:?}", e),
            Self::BadString(e) => write!(f, "{}", e),
            Self::IntParse(e) => write!(f, "{}", e),
            Self::NotSupported(cmd) => {
                write!(f, "command {} not supported by redisless", cmd)
            }
            Self::ProtocolParse(err) => write!(f, "{}", err),
            Self::InvalidCommand => write!(f, "invalid command"),
            Self::CommandNotFound => write!(f, "command not found"),
            Self::WrongTypeOperation => write!(
                f,
                "WRONGTYPE Operation against a key holding the wrong kind of value"
            ),
        }
    }
}

impl From<TimeOverflow> for RedisCommandError {
    fn from(err: TimeOverflow) -> Self {
        Self::TimeOverflow(err)
    }
}

impl From<Utf8Error> for RedisCommandError {
    fn from(err: Utf8Error) -> Self {
        Self::BadString(err)
    }
}

impl From<ParseIntError> for RedisCommandError {
    fn from(err: ParseIntError) -> Self {
        Self::IntParse(err)
    }
}
