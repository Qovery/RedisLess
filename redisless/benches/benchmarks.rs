use std::io::{Read, Write};
use std::net::TcpStream;

use criterion::{criterion_group, criterion_main, Criterion};
use redisless::{RedisLess, Server, ServerState};

fn criterion_benchmarks(c: &mut Criterion) {
    let port = 3335;
    let server = Server::new(RedisLess::new(), port);
    assert_eq!(server.start(), Some(ServerState::Started));

    let mut stream = TcpStream::connect(format!("localhost:{}", port)).unwrap();

    c.bench_function("set, get and del", |b| {
        b.iter(|| {
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
        });
    });

    assert_eq!(server.stop(), Some(ServerState::Stopped));
}

criterion_group!(benches, criterion_benchmarks);
criterion_main!(benches);
