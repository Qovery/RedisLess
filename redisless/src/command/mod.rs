#[cfg(test)]
mod tests;

pub mod command_error;
mod util;

use crate::protocol::Resp;
use crate::storage::models::Expiry;
use command_error::RedisCommandError;

use super::storage::models::RedisString;

type Key = RedisString;
type Value = RedisString;
type Items = Vec<(Key, Value)>;
type Keys = Vec<Key>;
type Values = Vec<Value>;

#[derive(Debug, PartialEq)]
pub enum Command {
    Append(Key, Value),
    Set(Key, Value),
    Setnx(Key, Value),
    Setex(Key, Expiry, Value),
    PSetex(Key, Expiry, Value),
    MSet(Items),
    MSetnx(Items),
    Expire(Key, Expiry),
    PExpire(Key, Expiry),
    Get(Key),
    GetSet(Key, Value),
    MGet(Keys),
    HSet(Key, Items),
    HGet(Key, Key),
    RPush(Key, Values),
    LPush(Key, Values),
    LLen(Key),
    RPushx(Key, Values),
    LPushx(Key, Values),
    Del(Key),
    Incr(Key),
    IncrBy(Key, i64),
    Exists(Key),
    Type(Key),
    Ttl(Key),
    Pttl(Key),
    Info,
    Ping,
    Quit,
    Dbsize,
}

impl Command {
    pub fn parse(v: Vec<Resp>) -> Result<Self, RedisCommandError> {
        use util::*;
        use Command::*;
        use RedisCommandError::*;

        match v.first() {
            Some(Resp::BulkString(command)) => match *command {
                b"SET" | b"set" | b"Set" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let value = get_bytes_vec(v.get(2))?;

                    Ok(Set(key, value))
                }
                b"APPEND" | b"append" | b"Append" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let value = get_bytes_vec(v.get(2))?;

                    Ok(Append(key, value))
                }
                b"SETEX" | b"setex" | b"SetEx" | b"Setex" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let duration = get_bytes_vec(v.get(2)).and_then(parse_duration)?;
                    let value = get_bytes_vec(v.get(3))?;
                    let expiry = Expiry::new_from_secs(duration)?;

                    Ok(Setex(key, expiry, value))
                }
                b"PSETEX" | b"psetex" | b"PSetEx" | b"PSetex" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let duration = get_bytes_vec(v.get(2)).and_then(parse_duration)?;
                    let value = get_bytes_vec(v.get(3))?;
                    let expiry = Expiry::new_from_millis(duration)?;

                    Ok(PSetex(key, expiry, value))
                }
                b"MSET" | b"MSet" | b"mset" => {
                    // Will not panic with out of bounds, because request has at least length 1,
                    // in which case request will be an empty slice
                    // &[key, value, key, value, key, value, ...] should be even in length
                    // We want [(key, value), (key, value), (key, value), ..]
                    let pairs = &v[1..];
                    let chunk_size = 2_usize;
                    if pairs.is_empty() || pairs.len() % chunk_size != 0 {
                        return Err(ArgNumber);
                    }

                    let mut items = Vec::<(Key, Value)>::with_capacity(pairs.len());
                    for pair in pairs.chunks_exact(chunk_size) {
                        match pair {
                            [key, value] => {
                                let key = get_bytes_vec(Some(&key))?;
                                let value = get_bytes_vec(Some(&value))?;
                                items.push((key, value));
                            }
                            _ => unreachable!(), // pairs has even length so each chunk will have len 2
                        }
                    }
                    Ok(MSet(items))
                }
                b"MSETNX" | b"MSetnx" | b"msetnx" => {
                    let pairs = &v[1..];

                    let chunk_size = 2_usize;
                    if pairs.is_empty() || pairs.len() % chunk_size != 0 {
                        return Err(ArgNumber);
                    }

                    let mut items = Items::with_capacity(pairs.len());
                    for pair in pairs.chunks_exact(chunk_size) {
                        match pair {
                            [key, value] => {
                                let key = get_bytes_vec(Some(&key))?;
                                let value = get_bytes_vec(Some(&value))?;
                                items.push((key, value));
                            }
                            _ => unreachable!(),
                        }
                    }

                    Ok(MSetnx(items))
                }
                b"SETNX" | b"setnx" | b"Setnx" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let value = get_bytes_vec(v.get(2))?;

                    Ok(Setnx(key, value))
                }
                b"EXPIRE" | b"expire" | b"Expire" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let duration = get_bytes_vec(v.get(2)).and_then(parse_duration)?;
                    let expiry = Expiry::new_from_secs(duration)?;

                    Ok(Expire(key, expiry))
                }
                b"PEXPIRE" | b"Pexpire" | b"PExpire" | b"pexpire" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let duration = get_bytes_vec(v.get(2)).and_then(parse_duration)?;
                    let expiry = Expiry::new_from_millis(duration)?;

                    Ok(PExpire(key, expiry))
                }
                b"GET" | b"get" | b"Get" => {
                    let key = get_bytes_vec(v.get(1))?;
                    Ok(Get(key))
                }
                b"GETSET" | b"getset" | b"Getset" | b"GetSet" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let value = get_bytes_vec(v.get(2))?;

                    Ok(GetSet(key, value))
                }
                b"MGET" | b"mget" | b"MGet" => {
                    let keys = &v[1..]; // will never panic
                    if keys.is_empty() {
                        return Err(ArgNumber);
                    }

                    let mut keys_vec = Keys::with_capacity(keys.len());
                    for key in keys {
                        let key = get_bytes_vec(Some(key))?;
                        keys_vec.push(key);
                    }

                    Ok(MGet(keys_vec))
                }
                b"HSET" | b"hset" | b"HMSET" | b"hmset" => {
                    let hash_key = get_bytes_vec(v.get(1))?;
                    let pairs = &v[2..];

                    let chunk_size = 2_usize;
                    if pairs.is_empty() || pairs.len() % chunk_size != 0 {
                        return Err(ArgNumber);
                    }

                    let mut items = Items::with_capacity(pairs.len());
                    for pair in pairs.chunks_exact(chunk_size) {
                        match pair {
                            [key, value] => {
                                let key = get_bytes_vec(Some(&key))?;
                                let value = get_bytes_vec(Some(&value))?;
                                items.push((key, value));
                            }
                            _ => unreachable!(),
                        }
                    }
                    Ok(HSet(hash_key, items))
                }
                b"HGET" | b"hget" => {
                    //HGet(Key, Key),
                    let hash_key = get_bytes_vec(v.get(1))?;
                    let field_key = get_bytes_vec(v.get(2))?;

                    Ok(HGet(hash_key, field_key))
                }
                b"RPUSH" | b"RPush" | b"Rpush" | b"rpush" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let values = &v[2..];

                    let mut values_vec = Values::with_capacity(values.len());
                    for value in values {
                        let value = get_bytes_vec(Some(value))?;
                        values_vec.push(value);
                    }

                    Ok(RPush(key, values_vec))
                }
                b"LPUSH" | b"LPush" | b"Lpush" | b"lpush" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let values = &v[2..];

                    let mut values_vec = Values::with_capacity(values.len());
                    for value in values {
                        let value = get_bytes_vec(Some(value))?;
                        values_vec.push(value);
                    }

                    Ok(LPush(key, values_vec))
                }
                b"LLEN" | b"LLen" | b"Llen" | b"llen" => {
                    let key = get_bytes_vec(v.get(1))?;
                    Ok(LLen(key))
                }
                b"RPUSHX" | b"RPushx" | b"Rpushx" | b"rpushx" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let values = &v[2..];

                    let mut values_vec = Values::with_capacity(values.len());
                    for value in values {
                        let value = get_bytes_vec(Some(value))?;
                        values_vec.push(value);
                    }
                    Ok(RPushx(key, values_vec))
                }
                b"LPUSHX" | b"LPushx" | b"Lpushx" | b"lpushx" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let values = &v[2..];

                    let mut values_vec = Values::with_capacity(values.len());
                    for value in values {
                        let value = get_bytes_vec(Some(value))?;
                        values_vec.push(value);
                    }
                    Ok(LPushx(key, values_vec))
                }
                b"DEL" | b"del" | b"Del" => {
                    let key = get_bytes_vec(v.get(1))?;
                    Ok(Del(key))
                }
                b"INCR" | b"incr" | b"Incr" => {
                    let key = get_bytes_vec(v.get(1))?;
                    Ok(Incr(key))
                }
                b"INCRBY" | b"incrby" | b"IncrBy" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let increment = get_bytes_vec(v.get(2)).and_then(parse_variation)?;
                    Ok(IncrBy(key, increment))
                }
                b"DECR" | b"decr" | b"Decr" => {
                    let key = get_bytes_vec(v.get(1))?;
                    Ok(IncrBy(key, -1))
                }
                b"DECRBY" | b"decrby" | b"DecrBy" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let decrement = get_bytes_vec(v.get(2)).and_then(parse_variation)?;
                    Ok(IncrBy(key, -decrement))
                }
                b"EXISTS" | b"exists" | b"Exists" => {
                    let key = get_bytes_vec(v.get(1))?;
                    Ok(Exists(key))
                }
                b"TYPE" | b"type" | b"Type" => {
                    let key = get_bytes_vec(v.get(1))?;
                    Ok(Type(key))
                }
                b"TTL" | b"ttl" | b"Ttl" => {
                    let key = get_bytes_vec(v.get(1))?;
                    Ok(Ttl(key))
                }
                b"PTTL" | b"pttl" | b"Pttl" => {
                    let key = get_bytes_vec(v.get(1))?;
                    Ok(Pttl(key))
                }
                b"INFO" | b"info" | b"Info" => Ok(Info),
                b"PING" | b"ping" | b"Ping" => Ok(Ping),
                b"DBSIZE" | b"dbsize" | b"Dbsize" => Ok(Dbsize),
                b"QUIT" | b"quit" | b"Quit" => Ok(Quit),
                unsupported_command => Err(NotSupported(
                    std::str::from_utf8(unsupported_command)
                        .unwrap()
                        .to_string(),
                )),
            },
            _ => Err(InvalidCommand),
        }
    }
}
