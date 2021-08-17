use prost::bytes::BufMut;

use super::{NIL, OK, PONG};
use crate::{command::command_error::RedisCommandError, storage::models::RedisString};

pub enum RedisResponseType {
    SimpleString(RedisString),
    BulkString(RedisString),
    Integer(i64),
    Nil,
}

pub struct RedisResponse {
    responses: RedisResponseInner,
}

enum RedisResponseInner {
    Single(RedisResponseType),
    Array(Vec<RedisResponseType>),
    Error(RedisCommandError),
    Okay,
    Pong,
    Quit,
}

impl RedisResponseType {
    // move out of the enum
    fn to_vec(&self) -> Vec<u8> {
        use RedisResponseType::*;
        match self {
            SimpleString(s) | BulkString(s) => s.clone(),
            Integer(num) => num.to_string().as_bytes().to_vec(),
            Nil => NIL.to_vec(),
        }
    }
    /// Move out of self and return bytes analogous to `format!("{}{}{}", symbol, data, CRLF)`
    pub fn get_formatted(self) -> Vec<u8> {
        use RedisResponseType::*;

        let symbol = match &self {
            SimpleString(_) => b'+',
            BulkString(_) => b'$',
            Integer(_) => b':',
            Nil => return self.to_vec(),
        };
        let mut bytes = self.to_vec();
        let mut reply =
            Vec::<u8>::with_capacity(bytes.len() + 3 /* 3 more bytes for symbol and /r/n */);
        reply.push(symbol);
        if symbol == b'$' {
            reply.put_slice(bytes.len().to_string().as_bytes());
            reply.put_slice(b"\r\n");
        }
        //eprintln!("{:?}", bytes);
        reply.append(&mut bytes);
        reply.put_slice(b"\r\n");
        reply
    }
}

impl RedisResponse {
    pub fn okay() -> Self {
        Self {
            responses: RedisResponseInner::Okay,
        }
    }
    pub fn pong() -> Self {
        Self {
            responses: RedisResponseInner::Pong,
        }
    }
    pub fn quit() -> Self {
        Self {
            responses: RedisResponseInner::Quit,
        }
    }
    pub fn is_quit(&self) -> bool {
        matches!(self.responses, RedisResponseInner::Quit)
    }

    pub fn single(response: RedisResponseType) -> Self {
        Self {
            responses: RedisResponseInner::Single(response),
        }
    }

    pub fn array(responses: Vec<RedisResponseType>) -> Self {
        Self {
            responses: RedisResponseInner::Array(responses),
        }
    }

    pub fn error(error: RedisCommandError) -> Self {
        Self {
            responses: RedisResponseInner::Error(error),
        }
    }

    pub fn reply(self) -> Vec<u8> {
        use RedisResponseInner::*;
        match self.responses {
            Okay | Quit => OK.to_vec(),
            Error(e) => e.to_vec(),
            Pong => PONG.to_vec(),
            Single(single) => single.get_formatted(),
            Array(responses) => {
                let mut reply = Vec::<u8>::with_capacity(512);
                reply.push(b'*');
                reply.put_slice(&responses.len().to_string().as_bytes().to_vec());
                reply.put_slice(b"\r\n");
                for response in responses {
                    let mut response = response.get_formatted();
                    reply.append(&mut response);
                }
                reply
            }
        }
    }
}
