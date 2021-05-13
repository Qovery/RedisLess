#[cfg(test)]
#[macro_use]
extern crate serial_test;

use storage::in_memory::InMemoryStorage;

use crate::server::{Server, ServerState};

mod cluster;
mod command;
mod error;
mod protocol;
pub mod server;
pub mod storage;

#[no_mangle]
pub unsafe extern "C" fn redisless_server_new(port: u16) -> *mut Server {
    Box::into_raw(Box::new(Server::new(InMemoryStorage::new(), port)))
}

#[no_mangle]
pub unsafe extern "C" fn redisless_server_free(server: *mut Server) {
    let _ = Box::from_raw(server);
}

#[no_mangle]
pub unsafe extern "C" fn redisless_server_start(server: *mut Server) -> bool {
    let server = match server.as_ref() {
        Some(server) => server,
        None => return false,
    };

    match server.start() {
        Some(server_state) => server_state == ServerState::Started,
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn redisless_server_stop(server: *mut Server) -> bool {
    let server = match server.as_ref() {
        Some(server) => server,
        None => return false,
    };

    match server.stop() {
        Some(server_state) => server_state == ServerState::Stopped,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    use crate::{
        redisless_server_free, redisless_server_new, redisless_server_start, redisless_server_stop,
    };

    #[test]
    #[serial]
    fn start_and_stop_server_from_c_binding() {
        let port = 4444 as u16;
        let server = unsafe { redisless_server_new(port) };

        unsafe {
            assert!(redisless_server_start(server), "server didn't start");
        }

        let mut stream = TcpStream::connect(format!("localhost:{}", port)).unwrap();

        for _ in 0..9 {
            // run command `PING`
            let _ = stream.write(b"*1\r\n$4\r\nPING\r\n");
            let mut pong_res = [0; 7];
            let _ = stream.read(&mut pong_res);
            assert_eq!(pong_res, b"+PONG\r\n"[..]);

            // run command `SET mykey value`
            let _ = stream.write(b"*3\r\n$3\r\nSET\r\n$5\r\nmykey\r\n$5\r\nvalue\r\n");
            let mut set_res = [0; 5];
            let _ = stream.read(&mut set_res);
            assert_eq!(set_res, b"+OK\r\n"[..]);

            // run command `GET mykey`
            let _ = stream.write(b"*2\r\n$3\r\nGET\r\n$5\r\nmykey\r\n");
            let mut get_res = [0; 8];
            let _ = stream.read(&mut get_res);
            assert_eq!(get_res, b"+value\r\n"[..]);

            // run command `DEL mykey`
            let _ = stream.write(b"*2\r\n$3\r\nDEL\r\n$5\r\nmykey\r\n");
            let mut del_res = [0; 4];
            let _ = stream.read(&mut del_res);
            assert_eq!(del_res, b":1\r\n"[..]);

            // run command `INFO`
            let _ = stream.write(b"*1\r\n$4\r\nINFO\r\n");
            let mut info_res = [0; 6];
            let _ = stream.read(&mut info_res);
            assert_eq!(info_res, b"$0\r\n\r\n"[..]);
        }

        unsafe {
            assert!(redisless_server_stop(server), "server didn't stop");
            redisless_server_free(server);
        }
    }
}
