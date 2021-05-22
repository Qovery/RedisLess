use super::command_error::RedisCommandError;
use crate::protocol::Resp;

pub fn get_bytes_vec(resp: Option<&Resp>) -> Result<Vec<u8>, RedisCommandError> {
    match resp {
        Some(Resp::String(x)) | Some(Resp::BulkString(x)) => Ok(x.to_vec()),
        _ => Err(RedisCommandError::ArgNumber),
    }
}

pub fn parse_duration(bytes: Vec<u8>) -> Result<u64, RedisCommandError> {
    let duration = std::str::from_utf8(&bytes[..])?;
    Ok(duration.parse::<u64>()?)
}

pub fn parse_variation(bytes: Vec<u8>) -> Result<i64, RedisCommandError> {
    let delta = std::str::from_utf8(&bytes[..])?;
    Ok(delta.parse::<i64>()?)
}
