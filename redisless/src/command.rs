use storage::in_memory::Expiry;

use crate::command::Command::{Error, NotSupported};
use crate::protocol::Resp;

type Key = Vec<u8>;
type Value = Vec<u8>;
type Message = &'static str;

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
    Error(Message),
    NotSupported(String),
}

fn get_bytes_vec(resp: Option<&Resp>) -> Option<Vec<u8>> {
    match resp {
        Some(Resp::String(x)) | Some(Resp::BulkString(x)) => Some(x.to_vec()),
        _ => None,
    }
}

impl Command {
    pub fn parse(v: Vec<Resp>) -> Self {
        match v.first() {
            Some(Resp::BulkString(op)) => match *op {
                b"SET" | b"set" | b"Set" => {
                    if v.len() != 3 {
                        return Error("wrong number of arguments for 'SET' command");
                    }

                    if let Some(key) = get_bytes_vec(v.get(1)) {
                        if let Some(value) = get_bytes_vec(v.get(2)) {
                            return Command::Set(key, value);
                        }
                    }

                    Error("wrong number of arguments for 'SET' command")
                }
                b"SETEX" | b"setex" | b"SetEx" | b"Setex" => {
                    if v.len() != 4 {
                        return Error("wrong number of arguments for 'SETEX' command");
                    }
                    if let Some(key) = get_bytes_vec(v.get(1)) {
                        // Redis sends value as index 3 and duration as index 2
                        if let Some(value) = get_bytes_vec(v.get(3)) {
                            if let Some(duration_bytes) = get_bytes_vec(v.get(2)) {
                                if let Ok(duration_str) = std::str::from_utf8(&duration_bytes[..]) {
                                    if let Ok(duration) = duration_str.parse::<u64>() {
                                        if let Some(expiry) = Expiry::new_from_secs(duration) {
                                            return Command::Setex(key, expiry, value);
                                        } else {
                                            return Error("too many seconds");
                                        }
                                    } else {
                                        return Error("could not parse duration as u64");
                                    }
                                } else {
                                    return Error("bad string in request");
                                }
                            }
                        }
                    }

                    Error("wrong number of arguments for 'SETEX' command")
                }
                b"EXPIRE" | b"expire" | b"Expire" => {
                    if v.len() != 3 {
                        return Error("wrong number of arguments for 'EXPIRE' command");
                    }

                    if let Some(key) = get_bytes_vec(v.get(1)) {
                        if let Some(duration_bytes) = get_bytes_vec(v.get(2)) {
                            if let Ok(duration_str) = std::str::from_utf8(&duration_bytes[..]) {
                                if let Ok(duration) = duration_str.parse::<u64>() {
                                    if let Some(expiry) = Expiry::new_from_secs(duration) {
                                        return Command::Expire(key, expiry);
                                    } else {
                                        return Error("too many seconds");
                                    }
                                } else {
                                    return Error("could not parse duration as u64");
                                }
                            } else {
                                return Error("bad string in request");
                            }
                        }
                    }

                    Error("wrong number of arguments for 'EXPIRE' command")
                }
                b"PEXPIRE" | b"Pexpire" | b"PExpire" => {
                    if v.len() != 3 {
                        return Error("wrong number of arguments for 'PEXPIRE' command");
                    }

                    if let Some(key) = get_bytes_vec(v.get(1)) {
                        if let Some(duration_bytes) = get_bytes_vec(v.get(2)) {
                            if let Ok(duration_str) = std::str::from_utf8(&duration_bytes[..]) {
                                if let Ok(duration) = duration_str.parse::<u64>() {
                                    if let Some(expiry) = Expiry::new_from_millis(duration) {
                                        return Command::PExpire(key, expiry);
                                    } else {
                                        return Error("too many milliseconds");
                                    }
                                } else {
                                    return Error("could not parse duration as u64");
                                }
                            } else {
                                return Error("bad string in request");
                            }
                        }
                    }

                    Error("wrong number of arguments for 'PEXPIRE' command")
                }
                b"GET" | b"get" | b"Get" => {
                    if v.len() != 2 {
                        return Error("wrong number of arguments for 'GET' command");
                    }

                    if let Some(arg1) = get_bytes_vec(v.get(1)) {
                        return Command::Get(arg1);
                    }

                    Error("wrong number of arguments for 'GET' command")
                }
                b"GETSET" | b"getset" | b"Getset" | b"GetSet" => {
                    if v.len() != 3 {
                        return Error("wrong number of arguments for 'GETSET' command");
                    }

                    if let Some(key) = get_bytes_vec(v.get(1)) {
                        if let Some(value) = get_bytes_vec(v.get(2)) {
                            return Command::GetSet(key, value);
                        }
                    }

                    Error("wrong number of arguments for 'SET' command")
                }
                b"DEL" | b"del" | b"Del" => {
                    if v.len() != 2 {
                        return Error("wrong number of arguments for 'DEL' command");
                    }

                    if let Some(arg1) = get_bytes_vec(v.get(1)) {
                        return Command::Del(arg1);
                    }

                    Error("wrong number of arguments for 'DEL' command")
                }
                b"INCR" | b"incr" | b"Incr" => {
                    if v.len() != 2 {
                        return Error("wrong number of arguments for 'INCR' command");
                    }

                    if let Some(arg1) = get_bytes_vec(v.get(1)) {
                        return Command::Incr(arg1);
                    }

                    Error("wrong number of arguments for 'INCR' command")
                }
                b"INFO" | b"info" | b"Info" => Command::Info,
                b"PING" | b"ping" | b"Ping" => Command::Ping,
                b"QUIT" | b"quit" | b"Quit" => Command::Quit,
                cmd => NotSupported(format!(
                    "Command '{}' is not supported",
                    std::str::from_utf8(cmd).unwrap()
                )),
            },
            _ => Error("Invalid command to parse"),
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

            let command = Command::parse(resp);

            assert_eq!(command, Command::Set(b"mykey".to_vec(), b"value".to_vec()));
        }
    }
}
