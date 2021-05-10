use crate::protocol::Resp;
use storage::in_memory::Expiry;

type Key = Vec<u8>;
type Value = Vec<u8>;

#[derive(Debug, PartialEq)]
pub enum Command {
    Set(Key, Value),
    Setex(Key, Expiry, Value),
    Expire(Key, Expiry),
    PExpire(Key, Expiry),
    Get(Key),
    GetSet(Key, Value),
    Del(Key),
    Incr(Key),
    Info,
    Ping,
    Quit,
}

fn get_bytes_vec(resp: Option<&Resp>) -> Option<Vec<u8>> {
    match resp {
        Some(Resp::String(x)) | Some(Resp::BulkString(x)) => Some(x.to_vec()),
        _ => None,
    }
}

use super::protocol::error::RedisCommandError;

impl Command {
    pub fn parse(v: Vec<Resp>) -> Result<Self, RedisCommandError> {
        use RedisCommandError::*;
        match v.first() {
            Some(Resp::BulkString(op)) => match *op {
                // Reorganize ?
                b"SET" | b"set" | b"Set" => {
                    if v.len() == 3 {
                        // Safe to unwrap index 1 and 2 if previously checked length
                        let key = get_bytes_vec(v.get(1)).unwrap();
                        let value = get_bytes_vec(v.get(2)).unwrap();
                        Ok(Command::Set(key, value))
                    } else {
                        Err(ArgNumber(format!("{:?}", op)))
                    }
                }
                b"SETEX" | b"setex" | b"SetEx" | b"Setex" => {
                    if v.len() == 4 {
                        let key = get_bytes_vec(v.get(1)).unwrap();
                        // Redis sends value as index 3 and duration as index 2
                        let value = get_bytes_vec(v.get(3)).unwrap();

                        // Might wanna add a parse duration function
                        let duration = get_bytes_vec(v.get(2)).unwrap();
                        let duration = std::str::from_utf8(&duration[..])?;
                        let duration = duration.parse::<u64>()?;
                        let expiry = Expiry::new_from_secs(duration)?;

                        Ok(Command::Setex(key, expiry, value))
                    } else {
                        Err(ArgNumber(format!("{:?}", op)))
                    }
                }
                b"EXPIRE" | b"expire" | b"Expire" => {
                    if v.len() == 3 {
                        let key = get_bytes_vec(v.get(1)).unwrap();

                        // Might wanna add a parse duration function
                        let duration = get_bytes_vec(v.get(2)).unwrap();
                        let duration = std::str::from_utf8(&duration[..])?;
                        let duration = duration.parse::<u64>()?;
                        let expiry = Expiry::new_from_secs(duration)?;

                        Ok(Command::Expire(key, expiry))
                    } else {
                        Err(ArgNumber(format!("{:?}", op)))
                    }
                }
                b"PEXPIRE" | b"Pexpire" | b"PExpire" | b"pexpire" => {
                    if v.len() == 3 {
                        let key = get_bytes_vec(v.get(1)).unwrap();

                        // Might wanna add a parse duration function
                        let duration = get_bytes_vec(v.get(2)).unwrap();
                        let duration = std::str::from_utf8(&duration[..])?;
                        let duration = duration.parse::<u64>()?;
                        let expiry = Expiry::new_from_millis(duration)?;

                        Ok(Command::PExpire(key, expiry))
                    } else {
                        Err(ArgNumber(format!("{:?}", op)))
                    }
                }
                b"GET" | b"get" | b"Get" => {
                    if v.len() == 2 {
                        let key = get_bytes_vec(v.get(1)).unwrap();
                        Ok(Command::Get(key))
                    } else {
                        Err(ArgNumber(format!("{:?}", op)))
                    }
                }
                b"GETSET" | b"getset" | b"Getset" | b"GetSet" => {
                    if v.len() == 3 {
                        let key = get_bytes_vec(v.get(1)).unwrap();
                        let value = get_bytes_vec(v.get(2)).unwrap();
                        Ok(Command::GetSet(key, value))
                    } else {
                        Err(ArgNumber(format!("{:?}", op)))
                    }
                }
                b"DEL" | b"del" | b"Del" => {
                    if v.len() == 2 {
                        let key = get_bytes_vec(v.get(1)).unwrap();
                        Ok(Command::Del(key))
                    } else {
                        Err(ArgNumber(format!("{:?}", op)))
                    }
                }
                b"INCR" | b"incr" | b"Incr" => {
                    if v.len() == 2 {
                        let key = get_bytes_vec(v.get(1)).unwrap();
                        Ok(Command::Incr(key))
                    } else {
                        Err(ArgNumber(format!("{:?}", op)))
                    }
                }
                b"INFO" | b"info" | b"Info" => Ok(Command::Info),
                b"PING" | b"ping" | b"Ping" => Ok(Command::Ping),
                b"QUIT" | b"quit" | b"Quit" => Ok(Command::Quit),
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
