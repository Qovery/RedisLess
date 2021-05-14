mod run_command;
// re-export run_command
use crossbeam_channel::{Receiver, Sender};
pub use run_command::*;

use crate::server::ServerState;

use std::{
    io::{BufReader, Read, Write},
    net::TcpStream,
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::Duration,
};

use crate::{
    command::{command_error::RedisCommandError, Command},
    protocol::{self, parser::RedisProtocolParser, Resp},
    storage::Storage,
};

use super::{CloseConnection, CommandResponse, ReceivedDataLength};

pub fn lock_then_release<T: Storage>(storage: &Arc<Mutex<T>>) -> MutexGuard<T> {
    loop {
        match storage.lock() {
            Ok(storage) => {
                return storage;
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

pub fn stop_sig_received(recv: &Receiver<ServerState>, sender: &Sender<ServerState>) -> bool {
    if let Ok(recv_state) = recv.try_recv() {
        if recv_state == ServerState::Stop {
            // notify that the server has been stopped
            let _ = sender.send(ServerState::Stopped);
            return true;
        }
    }

    false
}

pub fn get_command(bytes: &[u8; 512]) -> Result<Command, RedisCommandError> {
    match RedisProtocolParser::parse(bytes) {
        Ok((Resp::Array(v), _)) => match Command::parse(v) {
            Ok(command) => Ok(command),
            Err(err) => Err(err),
        },
        Err(err) => Err(RedisCommandError::ProtocolParse(err)),
        _ => Err(RedisCommandError::CommandNotFound),
    }
}

fn get_bytes_from_request(stream: &TcpStream) -> ([u8; 512], usize) {
    let mut buf_reader = BufReader::new(stream);
    let mut buf = [0; 512];
    let mut buf_length = 0_usize;

    while let Ok(s) = buf_reader.read(&mut buf) {
        buf_length += s;

        if s < 512 {
            break;
        }
    }

    (buf, buf_length)
}

pub fn handle_request<T: Storage>(
    storage: &Arc<Mutex<T>>,
    mut stream: &TcpStream,
) -> (CloseConnection, ReceivedDataLength) {
    let (buf, buf_length) = get_bytes_from_request(stream);

    match buf.get(0) {
        Some(x) if *x == 0 => {
            return (false, buf_length);
        }
        _ => {}
    }

    let (command, res) = run_command_and_get_response(storage, &buf);

    let _ = stream.write(res.as_slice());

    match command {
        Some(command) if command == Command::Quit => (true, buf_length),
        _ => (false, buf_length),
    }
}
