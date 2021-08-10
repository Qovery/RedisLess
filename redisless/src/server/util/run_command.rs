use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use chrono::format::format;

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
            Command::Append(k, v) => {
                let len = lock_then_release(storage).extend(k.as_slice(), v.as_slice());
                RedisResponse::single(Integer(len as i64))
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
            Command::RPush(key, values) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype != "list".as_bytes() && keytype != "none".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                let mut len = values.len();
                let mut new_vals = values.to_vec();
                match storage.lread(&key) {
                    Some(vals) => {
                        let mut vals = vals.to_vec();
                        vals.append(&mut new_vals);
                        len = vals.len();
                        storage.lwrite(&key, vals);
                        RedisResponse::single(Integer(len as i64))
                    }
                    None => {
                        storage.lwrite(&key, new_vals);
                        RedisResponse::single(Integer(len as i64))
                    }
                }
            }
            Command::LPush(key, values) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype != "list".as_bytes() && keytype != "none".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                let mut len = values.len();
                let mut values: Vec<RedisString> = values.to_vec().into_iter().rev().collect();
                match storage.lread(&key) {
                    Some(old_vals) => {
                        let mut old_vals = old_vals.to_vec();
                        values.append(&mut old_vals);
                        len = values.len();
                        storage.lwrite(&key, values);
                        RedisResponse::single(Integer(len as i64))
                    }
                    None => {
                        storage.lwrite(&key, values);
                        RedisResponse::single(Integer(len as i64))
                    }
                }
            }
            Command::LLen(key) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype != "list".as_bytes() && keytype != "none".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                match storage.lread(&key) {
                    Some(vals) => RedisResponse::single(Integer(vals.len() as i64)),
                    None => RedisResponse::single(Integer(0)),
                }
            }
            Command::RPushx(key, values) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype == "none".as_bytes() {
                    return RedisResponse::single(Integer(0));
                }
                if keytype != "list".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                let mut new_vals = values.to_vec();
                match storage.lread(&key) {
                    Some(vals) => {
                        let mut vals = vals.to_vec();
                        vals.append(&mut new_vals);
                        let len = vals.len();
                        storage.lwrite(&key, vals);
                        RedisResponse::single(Integer(len as i64))
                    }
                    None => RedisResponse::single(Integer(0)),
                }
            }
            Command::LPushx(key, values) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype == "none".as_bytes() {
                    return RedisResponse::single(Integer(0));
                }
                if keytype != "list".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                let mut values: Vec<RedisString> = values.to_vec().into_iter().rev().collect();
                match storage.lread(&key) {
                    Some(old_vals) => {
                        let mut old_vals = old_vals.to_vec();
                        values.append(&mut old_vals);
                        let len = values.len();
                        storage.lwrite(&key, values);
                        RedisResponse::single(Integer(len as i64))
                    }
                    None => RedisResponse::single(Integer(0)),
                }
            }
            Command::RPop(key) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype == "none".as_bytes() {
                    return RedisResponse::single(Nil);
                }
                if keytype != "list".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                match storage.lread(&key) {
                    Some(values) => {
                        let mut values = values.to_vec();
                        match values.pop() {
                            Some(value) => {
                                if values.is_empty() {
                                    storage.remove(&key);
                                } else {
                                    storage.lwrite(&key, values);
                                }
                                RedisResponse::single(BulkString(value))
                            }
                            None => RedisResponse::single(Nil),
                        }
                    }
                    None => RedisResponse::single(Nil),
                }
            }
            Command::LPop(key) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype == "none".as_bytes() {
                    return RedisResponse::single(Nil);
                }
                if keytype != "list".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                match storage.lread(&key) {
                    Some(values) => {
                        let mut values = values.to_vec();
                        let value = values.remove(0);
                        if values.is_empty() {
                            storage.remove(&key);
                        } else {
                            storage.lwrite(&key, values);
                        }
                        RedisResponse::single(BulkString(value))
                    }
                    None => RedisResponse::single(Nil),
                }
            }
            Command::LIndex(key, index) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype == "none".as_bytes() {
                    return RedisResponse::single(Nil);
                }
                if keytype != "list".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                let mut index = index;
                let values = storage.lread(&key).unwrap().to_vec();
                let len = values.len() as i64;
                if index < 0 {
                    index = index + len;
                }
                if index < 0 || index >= len {
                    return RedisResponse::single(Nil);
                }
                match values.get(index as usize) {
                    Some(value) => RedisResponse::single(SimpleString(value.to_vec())),
                    None => RedisResponse::single(Nil),
                }
            }
            Command::LSet(key, index, value) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype == "none".as_bytes() {
                    return RedisResponse::error(RedisCommandError::NoSuchKey);
                }
                if keytype != "list".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                let mut index = index;
                let mut values = storage.lread(&key).unwrap().to_vec();
                let len = values.len() as i64;
                if index < 0 {
                    index = index + len;
                }
                if index < 0 || index >= len {
                    return RedisResponse::error(RedisCommandError::IndexOutOfRange);
                }
                let _ = std::mem::replace(&mut values[index as usize], value);
                storage.lwrite(&key, values);
                RedisResponse::okay()
            }
            Command::LInsert(key, place, pivot, value) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype == "none".as_bytes() {
                    return RedisResponse::single(Integer(0));
                }
                if keytype != "list".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                if place != b"BEFORE" && place != b"AFTER" {
                    return RedisResponse::error(RedisCommandError::SyntaxErr);
                }
                let mut values = storage.lread(&key).unwrap().to_vec();
                let index = values.iter().position(|v| v == &pivot);
                match index {
                    Some(mut i) => {
                        if place == b"AFTER" {
                            i = i + 1;
                        }
                        values.insert(i, value);
                        let len = values.len();
                        storage.lwrite(&key, values);
                        RedisResponse::single(Integer(len as i64))
                    }
                    None => RedisResponse::single(Integer(-1)),
                }
            }
            Command::LTrim(key, start, stop) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype == "none".as_bytes() {
                    return RedisResponse::okay();
                }
                if keytype != "list".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                let mut values = storage.lread(&key).unwrap().to_vec();
                let len = values.len() as i64;
                let mut start = start;
                let mut stop = stop;
                if start < 0 {
                    start = start + len;
                }
                if stop < 0 {
                    stop = stop + len;
                }
                if start < 0 {
                    start = 0;
                }
                if stop < start || start > len {
                    storage.remove(&key);
                    return RedisResponse::okay();
                }
                stop = if stop >= len { len } else { stop + 1 };
                let vals: Vec<_> = values.drain(start as usize..stop as usize).collect();
                if vals.is_empty() {
                    storage.remove(&key);
                } else {
                    storage.lwrite(&key, vals);
                }
                RedisResponse::okay()
            }
            Command::LRem(key, count, value) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype == "none".as_bytes() {
                    return RedisResponse::single(Integer(0));
                }
                if keytype != "list".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                let values = storage.lread(&key).unwrap().to_vec();
                let len = values.len();
                let mut count = count;
                let mut vals = vec![];
                let mut rem = 0;
                if count < 0 {
                    for v in values.iter().rev() {
                        if *v == value && count < 0 {
                            count += 1;
                            rem += 1;
                            continue;
                        }
                        vals.push(v.clone());
                    }
                    vals = vals.into_iter().rev().collect();
                    storage.lwrite(&key, vals);
                    return RedisResponse::single(Integer(rem));
                }
                if count == 0 {
                    count = len as i64;
                }
                for v in values.iter() {
                    if *v == value && count > 0 {
                        count -= 1;
                        rem += 1;
                        continue;
                    }
                    vals.push(v.clone());
                }
                if vals.is_empty() {
                    storage.remove(&key);
                } else {
                    storage.lwrite(&key, vals);
                }
                RedisResponse::single(Integer(rem))
            }
            Command::RPopLPush(src, dest) => {
                let mut storage = lock_then_release(storage);
                let src_type = storage.type_of(&src);
                if src_type == "none".as_bytes() {
                    return RedisResponse::single(Nil);
                }
                if src_type != "list".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                let dest_type = storage.type_of(&dest);
                if dest_type != "list".as_bytes() && dest_type != "none".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                let mut src_values = storage.lread(&src).unwrap().to_vec();
                let mut dest_values = match storage.lread(&dest) {
                    Some(vals) => vals.to_vec(),
                    None => Vec::new(),
                };
                match src_values.pop() {
                    Some(val) => {
                        let value = val.clone();
                        dest_values.insert(0, val);
                        storage.lwrite(&dest, dest_values);
                        if src_values.is_empty() {
                            storage.remove(&src);
                        } else {
                            storage.lwrite(&src, src_values);
                        }
                        RedisResponse::single(BulkString(value))
                    }
                    None => RedisResponse::single(Nil),
                }
            }
            Command::SAdd(key, values) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype != "set".as_bytes() && keytype != "none".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                let mut len = values.len();
                match storage.sread(&key) {
                    Some(old_vals) => {
                        let diff: HashSet<_> = values.difference(old_vals).collect();
                        len = diff.len();
                        let vals: HashSet<_> = values.union(old_vals).cloned().collect();
                        storage.swrite(&key, vals);
                        RedisResponse::single(Integer(len as i64))
                    }
                    None => {
                        storage.swrite(&key, values);
                        RedisResponse::single(Integer(len as i64))
                    }
                }
            }
            Command::SCard(key) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype == "none".as_bytes() {
                    return RedisResponse::single(Integer(0));
                }
                if keytype != "set".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                let values = storage.sread(&key).unwrap();
                let len = values.len() as i64;
                RedisResponse::single(Integer(len))
            }
            Command::SRem(key, values) => {
                let mut storage = lock_then_release(storage);
                let keytype = storage.type_of(&key);
                if keytype == "none".as_bytes() {
                    return RedisResponse::single(Integer(0));
                }
                if keytype != "set".as_bytes() {
                    return RedisResponse::error(RedisCommandError::WrongTypeOperation);
                }
                let mut vals = storage.sread(&key).unwrap().to_owned();
                let mut rem = 0;
                for v in values {
                    if vals.remove(&v) {
                        rem = rem + 1;
                    }
                }
                storage.swrite(&key, vals);
                RedisResponse::single(Integer(rem))
            }
            Command::Del(k) => {
                let d = lock_then_release(storage).remove(k.as_slice());
                RedisResponse::single(Integer(d as i64))
            }
            Command::Incr(k) => {
                let mut storage = lock_then_release(storage);

                match storage.read(k.as_slice()) {
                    Some(value) => {
                        match std::str::from_utf8(value).unwrap().parse::<i64>() {
                            Ok(mut int_val) => {
                                int_val += 1;
                                let new_value = int_val.to_string().into_bytes();
                                storage.write(k.as_slice(), new_value.as_slice());
                                RedisResponse::single(Integer(int_val as i64))
                            }
                            Err(error) => RedisResponse::error(RedisCommandError::IntParse(error))
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
                        match std::str::from_utf8(value).unwrap().parse::<i64>() {
                            Ok(mut int_val) => {
                                int_val += increment;
                                let new_value = int_val.to_string().into_bytes();
                                storage.write(k.as_slice(), new_value.as_slice());
                                RedisResponse::single(Integer(int_val as i64))
                            }
                            Err(error) => RedisResponse::error(RedisCommandError::IntParse(error))
                        }
                    }
                    None => {
                        let val = increment.to_string();
                        storage.write(&k, val.as_bytes());
                        RedisResponse::single(Integer(increment))
                    }
                }
            }
            Command::Type(k) => {
                let mut s = lock_then_release(storage);
                let value_type = s.type_of(k.as_slice());
                RedisResponse::single(SimpleString(value_type.to_vec()))
            }
            Command::Exists(k) => {
                let exists = lock_then_release(storage).contains(&k);
                let exists: i64 = match exists {
                    true => 1,
                    false => 0,
                };
                RedisResponse::single(Integer(exists))
            }
            Command::Ttl(k) => {
                let ttl = if let Some(meta) = lock_then_release(storage).meta(&k) {
                    if let Some(expiry) = meta.expiry {
                        expiry.duration_left_millis() / 1000
                    } else {
                        -1
                    }
                } else {
                    -2
                };
                RedisResponse::single(Integer(ttl))
            }
            Command::Pttl(k) => {
                let ttl = if let Some(meta) = lock_then_release(storage).meta(&k) {
                    if let Some(expiry) = meta.expiry {
                        expiry.duration_left_millis()
                    } else {
                        -1
                    }
                } else {
                    -2
                };
                RedisResponse::single(Integer(ttl))
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
