use crate::resp::RESP;

type Key = String;
type Value = String;

pub enum Command {
    Set(Key, Value),
    Get(Key),
    Del(Key),
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
    pub fn parse(v: Vec<RESP>) -> Option<Self> {
        match v.first() {
            Some(RESP::BulkString(op)) => match *op {
                b"SET" => {
                    if let Some(arg1) = get_str(v.get(1)) {
                        if let Some(arg2) = get_str(v.get(2)) {
                            return Some(Command::Set(arg1, arg2));
                        }
                    }

                    None
                }
                b"GET" => {
                    if let Some(arg1) = get_str(v.get(1)) {
                        return Some(Command::Get(arg1));
                    }

                    None
                }
                b"DEL" => {
                    if let Some(arg1) = get_str(v.get(1)) {
                        return Some(Command::Del(arg1));
                    }

                    None
                }
                _ => None,
            },
            _ => None,
        }
    }
}
