use crate::command::Command::{Error, NotSupported};
use crate::resp::RESP;

type Key = Vec<u8>;
type Value = Vec<u8>;
type Message = &'static str;

pub enum Command {
    Set(Key, Value),
    Get(Key),
    Del(Key),
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
                b"SET" => {
                    if let Some(arg1) = get_bytes_vec(v.get(1)) {
                        if let Some(arg2) = get_bytes_vec(v.get(2)) {
                            return Command::Set(arg1, arg2);
                        }
                    }

                    Error("wrong number of arguments for 'SET' command")
                }
                b"GET" => {
                    if let Some(arg1) = get_bytes_vec(v.get(1)) {
                        return Command::Get(arg1);
                    }

                    Error("wrong number of arguments for 'GET' command")
                }
                b"DEL" => {
                    if let Some(arg1) = get_bytes_vec(v.get(1)) {
                        return Command::Del(arg1);
                    }

                    Error("wrong number of arguments for 'DEL' command")
                }
                cmd => NotSupported(format!(
                    "Command '{}' is not supported",
                    std::str::from_utf8(cmd).unwrap()
                )),
            },
            _ => Error("Invalid command to parse"),
        }
    }
}
