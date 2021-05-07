use crate::command::Command::{Error, NotSupported};
use crate::protocol::RESP;

type Key = Vec<u8>;
type Value = Vec<u8>;
type Message = &'static str;

#[derive(Debug, PartialEq)]
pub enum Command {
    Set(Key, Value),
    Setex(Key, Value, u64),
    Expire(Key, u64),
    Get(Key),
    Del(Key),
    Incr(Key),
    Info,
    Ping,
    Quit,
    Error(Message),
    NotSupported(String),
}

fn get_bytes_vec(resp: Option<&RESP>) -> Option<Vec<u8>> {
    match resp {
        Some(RESP::String(x)) | Some(RESP::BulkString(x)) => Some(x.to_vec()),
        _ => None,
    }
}

impl Command {
    pub fn parse(v: Vec<RESP>) -> Self {
        match v.first() {
            Some(RESP::BulkString(op)) => match *op {
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
                                        return Command::Setex(key, value, duration);
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
                                    return Command::Expire(key, duration);
                                } else {
                                    return Error("could not parse duration as u64");
                                }
                            } else {
                                return Error("bad string in request");
                            }
                        }
                    }

                    Error("wrong number of arguments for 'SETEX' command")
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
    use crate::protocol::RESP;

    #[test]
    fn set_command() {
        let commands = vec![b"SET", b"set"];

        for cmd in commands {
            let resp = vec![
                RESP::BulkString(cmd),
                RESP::BulkString(b"mykey"),
                RESP::BulkString(b"value"),
            ];

            let command = Command::parse(resp);

            assert_eq!(command, Command::Set(b"mykey".to_vec(), b"value".to_vec()));
        }
    }
}
