use crate::protocol::error::RedisCommandError;
use crate::protocol::Resp;
use storage::in_memory::Expiry;

type Key = Vec<u8>;
type Value = Vec<u8>;
type Items = Vec<(Key, Value)>;

#[derive(Debug, PartialEq)]
pub enum Command {
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
    Del(Key),
    Incr(Key),
    Exists(Key),
    Info,
    Ping,
    Quit,
}

fn get_bytes_vec(resp: Option<&Resp>) -> Result<Vec<u8>, RedisCommandError> {
    match resp {
        Some(Resp::String(x)) | Some(Resp::BulkString(x)) => Ok(x.to_vec()),
        _ => Err(RedisCommandError::ArgNumber),
    }
}

fn parse_duration(bytes: Vec<u8>) -> Result<u64, RedisCommandError> {
    let duration = std::str::from_utf8(&bytes[..])?;
    Ok(duration.parse::<u64>()?)
}

impl Command {
    pub fn parse(v: Vec<Resp>) -> Result<Self, RedisCommandError> {
        use Command::*;
        use RedisCommandError::*;

        match v.first() {
            Some(Resp::BulkString(command)) => match *command {
                b"SET" | b"set" | b"Set" => {
                    let key = get_bytes_vec(v.get(1))?;
                    let value = get_bytes_vec(v.get(2))?;

                    Ok(Set(key, value))
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
                    for pair in pairs.chunks(chunk_size) {
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

                    let mut items = Vec::<(Key, Value)>::with_capacity(pairs.len());
                    for pair in pairs.chunks(chunk_size) {
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
                b"DEL" | b"del" | b"Del" => {
                    let key = get_bytes_vec(v.get(1))?;
                    Ok(Del(key))
                }
                b"INCR" | b"incr" | b"Incr" => {
                    let key = get_bytes_vec(v.get(1))?;
                    Ok(Incr(key))
                }
                b"EXISTS" | b"exists" | b"Exists" => {
                    let key = get_bytes_vec(v.get(1))?;
                    Ok(Exists(key))
                }
                b"INFO" | b"info" | b"Info" => Ok(Info),
                b"PING" | b"ping" | b"Ping" => Ok(Ping),
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

#[cfg(test)]
mod tests {
    use crate::command::Command;
    use crate::protocol::Resp;

    #[test]
    fn set_command() {
        let commands = vec![b"SET", b"set"];
        for cmd in commands {
            let resp = vec![
                Resp::BulkString(cmd),
                Resp::BulkString(b"mykey"),
                Resp::BulkString(b"value"),
            ];

            let command = Command::parse(resp).unwrap();
            assert_eq!(command, Command::Set(b"mykey".to_vec(), b"value".to_vec()));
        }
    }
}
