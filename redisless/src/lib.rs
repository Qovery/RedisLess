#[cfg(test)]
#[macro_use]
extern crate serial_test;

use storage::in_memory::InMemoryStorage;

use crate::server::{Server, ServerState};

#[cfg(test)]
mod tests;

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
