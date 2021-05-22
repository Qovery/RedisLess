use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    command::Command,
    protocol::response::{RedisResponse, RedisResponseType},
    storage::{models::RedisString, Storage},
};

use super::*;

pub fn run_command_and_get_response<T: Storage>(
    storage: &Arc<Mutex<T>>,
    bytes: &[u8; 512],
) -> RedisResponse {
    use protocol::response::RedisResponseType::*;
    let command = get_command(bytes);
    let response = match command {
        Ok(command) => match command {
            Command::Set(k, v) => {
                lock_then_release(storage).write(k.as_slice(), v.as_slice());
                RedisResponse::okay()
            }
            Command::Setex(k, expiry, v) | Command::PSetex(k, expiry, v) => {
                let mut storage = lock_then_release(storage);

                storage.write(k.as_slice(), v.as_slice());
                storage.expire(k.as_slice(), expiry);

                RedisResponse::okay()
            }
            Command::Setnx(k, v) => {
                let mut storage = lock_then_release(storage);
                match storage.contains(&k[..]) {
                    // Key exists, will not re set key
                    true => RedisResponse::single(Integer(0)),
                    // Key does not exist, will set key
                    false => {
                        storage.write(&k, &v);
                        RedisResponse::single(Integer(1))
                    }
                }
            }
            Command::MSet(items) => {
                let mut storage = lock_then_release(storage);
                items.iter().for_each(|(k, v)| storage.write(k, v));
                RedisResponse::okay()
            }
            Command::MSetnx(items) => {
                // Either set all or not set any at all if any already exist
                let mut storage = lock_then_release(storage);
                match items.iter().all(|(key, _)| !storage.contains(key)) {
                    // None of the keys already exist in the storage
                    true => {
                        items.iter().for_each(|(k, v)| storage.write(k, v));
                        RedisResponse::single(Integer(1))
                    }
                    // Some key exists, don't write any of the keys
                    false => RedisResponse::single(Integer(0)),
                }
            }
            Command::Expire(k, expiry) | Command::PExpire(k, expiry) => {
                let e = lock_then_release(storage).expire(k.as_slice(), expiry);
                RedisResponse::single(Integer(e as i64))
            }
            Command::Get(k) => match lock_then_release(storage).read(k.as_slice()) {
                Some(value) => RedisResponse::single(SimpleString(value.to_vec())),
                None => RedisResponse::single(Nil),
            },
            Command::GetSet(k, v) => {
                let mut storage = lock_then_release(storage);

                let response = match storage.read(k.as_slice()) {
                    Some(value) => RedisResponse::single(SimpleString(value.to_vec())),
                    None => RedisResponse::single(Nil),
                };
                storage.write(k.as_slice(), v.as_slice());
                response
            }
            Command::MGet(keys) => {
                let mut storage = lock_then_release(storage);
                let mut responses = Vec::<RedisResponseType>::with_capacity(keys.len());
                for key in keys {
                    let response = match storage.read(key.as_slice()) {
                        Some(value) => RedisResponseType::SimpleString(value.to_vec()),
                        None => RedisResponseType::Nil,
                    };
                    responses.push(response);
                }
                RedisResponse::array(responses)
            }
            Command::HSet(map_key, items) => {
                let mut hash_map = HashMap::<RedisString, RedisString>::with_capacity(items.len());

                for (k, v) in items {
                    hash_map.insert(k.to_vec(), v.to_vec());
                }

                let mut storage = lock_then_release(storage);
                storage.hwrite(&map_key, hash_map);
                RedisResponse::okay()
            }
            Command::HGet(map_key, field_key) => {
                match lock_then_release(storage).hread(map_key.as_slice(), field_key.as_slice()) {
                    Some(value) => RedisResponse::single(SimpleString(value.to_vec())),
                    None => RedisResponse::single(Nil),
                }
            }
            Command::Del(k) => {
                let d = lock_then_release(storage).remove(k.as_slice());
                RedisResponse::single(Integer(d as i64))
            }
            Command::Incr(k) => {
                let mut storage = lock_then_release(storage);

                match storage.read(k.as_slice()) {
                    Some(value) => {
                        if let Ok(mut int_val) = std::str::from_utf8(value).unwrap().parse::<i64>()
                        {
                            int_val += 1;
                            let new_value = int_val.to_string().into_bytes();
                            storage.write(k.as_slice(), new_value.as_slice());
                            RedisResponse::single(Integer(int_val as i64))
                        } else {
                            // handle this error
                            unimplemented!()
                        }
                    }
                    None => {
                        let val = "1";
                        storage.write(&k, val.as_bytes());
                        RedisResponse::single(Integer(1))
                    }
                }
            }
            Command::IncrBy(k, increment) => {
                let mut storage = lock_then_release(storage);

                match storage.read(k.as_slice()) {
                    Some(value) => {
                        if let Ok(mut int_val) = std::str::from_utf8(value).unwrap().parse::<i64>()
                        {
                            int_val += increment;
                            let new_value = int_val.to_string().into_bytes();
                            storage.write(k.as_slice(), new_value.as_slice());
                            RedisResponse::single(Integer(int_val as i64))
                        } else {
                            //RedisResponse::error(...)
                            unimplemented!()
                        }
                    }
                    None => {
                        let val = increment.to_string();
                        storage.write(&k, val.as_bytes());
                        RedisResponse::single(Integer(increment))
                    }
                }
            }
            Command::Exists(k) => {
                let exists = lock_then_release(storage).contains(&k);
                let exists: i64 = match exists {
                    true => 1,
                    false => 0,
                };
                RedisResponse::single(Integer(exists))
            }
            Command::Info => RedisResponse::single(BulkString("".as_bytes().to_vec())),
            Command::Ping => RedisResponse::pong(),
            Command::Dbsize => {
                let storage = lock_then_release(storage);
                let size = storage.size() as i64;
                RedisResponse::single(Integer(size))
            }
            Command::Quit => RedisResponse::quit(),
        },
        Err(err) => RedisResponse::error(err),
    };
    response
}
