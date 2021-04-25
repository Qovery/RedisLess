use crate::command::Command::{Error, NotSupported};
use crate::resp::RESP;

type Key = String;
type Value = String;
type Message = String;

pub enum Command {
    Set(Key, Value),
    Get(Key),
    Del(Key),
    Error(Message),
    NotSupported(String),
}

fn get_str(resp: Option<&RESP>) -> Option<String> {
    match resp {
        Some(RESP::String(x)) | Some(RESP::BulkString(x)) => {
            Some(String::from_utf8(x.to_vec()).unwrap())
        }
        _ => None,
    }
}

impl Command {
    pub fn parse(v: Vec<RESP>) -> Self {
        match v.first() {
            Some(RESP::BulkString(op)) => match *op {
                b"SET" => {
                    if let Some(arg1) = get_str(v.get(1)) {
                        if let Some(arg2) = get_str(v.get(2)) {
                            return Command::Set(arg1, arg2);
                        }
                    }

                    Error(String::from("wrong number of arguments for 'SET' command"))
                }
                b"GET" => {
                    if let Some(arg1) = get_str(v.get(1)) {
                        return Command::Get(arg1);
                    }

                    Error(String::from("wrong number of arguments for 'GET' command"))
                }
                b"DEL" => {
                    if let Some(arg1) = get_str(v.get(1)) {
                        return Command::Del(arg1);
                    }

                    Error(String::from("wrong number of arguments for 'DEL' command"))
                }
                cmd => NotSupported(format!(
                    "Command '{}' is not supported",
                    std::str::from_utf8(cmd).unwrap()
                )),
            },
            _ => Error(String::from("Invalid command to parse")),
        }
    }
}
