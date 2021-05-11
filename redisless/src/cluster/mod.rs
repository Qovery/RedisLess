use std::collections::HashMap;
use std::io::{BufReader, Error, ErrorKind, Read};
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{unbounded, Receiver, RecvTimeoutError};

/// A Node represent a single RedisLess instance within a Cluster.
#[derive(Debug, Clone)]
pub struct Node {
    id: u64,
    socket_addr: SocketAddr,
}

impl Node {
    pub fn new(id: u64, socket_addr: SocketAddr) -> Self {
        Node { id, socket_addr }
    }

    pub fn listen(&self) -> Result<Receiver<()>, std::io::Error> {
        let listener = TcpListener::bind(self.socket_addr.to_string())?;
        let (sender, recv) = unbounded::<()>();

        let _ = thread::spawn(move || {
            let sender = sender;

            let thread_pool = match rayon::ThreadPoolBuilder::new()
                .thread_name(|_| "raft".to_string())
                .num_threads(4)
                .build()
            {
                Ok(thread_pool) => thread_pool,
                Err(err) => return Err(Error::new(ErrorKind::Other, err.to_string())),
            };

            for tcp_stream in listener.incoming() {
                let tcp_stream = tcp_stream?;
                let sender = sender.clone();
                // process incoming request

                let _ = thread_pool.spawn(move || {
                    let sender = sender.clone();

                    loop {
                        let mut buf_reader = BufReader::new(&tcp_stream);
                        let mut buf = [0; 512];
                        let mut buf_length = 0 as usize;

                        while let Ok(s) = buf_reader.read(&mut buf) {
                            buf_length += s;
                            if s < 512 {
                                break;
                            }
                        }

                        // TODO convert bytes to Message payload
                        // TODO and use sender.send(msg)
                    }
                });
            }

            Ok(())
        });

        Ok(recv)
    }

    pub fn send(&self, message: ()) {
        // TODO
    }
}

/// A Cluster represent nodes that are connected altogether and work as a single unit.
pub struct Cluster {
    current_node: Node,
    peer_nodes: Arc<Vec<Node>>,
}

impl Cluster {
    pub fn new(current_node: Node, peer_nodes: Vec<Node>) -> Self {
        Cluster {
            current_node,
            peer_nodes: Arc::new(peer_nodes),
        }
    }

    pub fn init(&self) -> Result<(), std::io::Error> {
        let receiver = self.current_node.listen()?;
        let peer_nodes = self.peer_nodes.clone();

        let _ = thread::spawn(move || {
            let mut now = Instant::now();
            let timeout = Duration::from_millis(100);
            let mut remaining_timeout = timeout;
            let peer_nodes = peer_nodes;

            loop {
                match receiver.recv_timeout(timeout) {
                    Ok(msg) => msg,
                    Err(RecvTimeoutError::Timeout) => (),
                    Err(RecvTimeoutError::Disconnected) => break,
                }

                let elapsed = now.elapsed();
                if elapsed >= timeout {
                    remaining_timeout = timeout;
                    // We drive Raft every 100ms.
                    // TODO raft tick
                } else {
                    remaining_timeout -= elapsed;
                }

                // TODO on_ready(..)
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::thread;
    use std::time::Duration;

    use rand::{thread_rng, RngCore};

    use crate::cluster::{Cluster, Node};

    #[test]
    fn start_and_stop_cluster() {
        let mut rng = thread_rng();

        let n1 = Node::new(
            rng.next_u64(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5555),
        );

        let n2 = Node::new(
            rng.next_u64(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5556),
        );

        let n3 = Node::new(
            rng.next_u64(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5557),
        );

        let _ = n2.listen();
        let _ = n3.listen();

        let cluster = Cluster::new(n1, vec![n2, n3]);

        let _ = cluster.init();
    }
}
