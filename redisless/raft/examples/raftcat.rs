//! A complex networked example as a command-line tool.

use bytes::{BufMut, Bytes};
use prost::Message as PMessage;
use raft::log::memory::InMemoryLog;
use raft::message::{Message, MessageDestination, SendableMessage};
use raft::node::{AppendError, Config, Node};
use rand_core::OsRng;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const TICK_DURATION: Duration = Duration::from_millis(50);
const RAFT_LOG_CAPACITY: usize = 100 * 1024 * 1024;
const RAFT_CONFIG: Config = Config {
    election_timeout_ticks: 10,
    heartbeat_interval_ticks: 5,
    replication_chunk_size: 65536,
};

type NodeId = String;

#[derive(Clone)]
enum IncomingMessage {
    Append(Bytes),
    Message(NetworkMessage),
}

#[derive(Clone, PMessage)]
pub struct NetworkMessage {
    #[prost(bytes, required)]
    pub from: Bytes,
    #[prost(message, required)]
    pub message: Message,
}

struct Network {
    peers_tx: BTreeMap<NodeId, mpsc::Sender<Message>>,
}

struct Args {
    bind_addr: Option<String>,
    node_id: NodeId,
    peers: BTreeSet<NodeId>,
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let Args {
        bind_addr,
        node_id,
        peers,
    } = parse_args();

    let (main_tx, main_rx) = mpsc::channel::<IncomingMessage>();
    if let Some(bind_addr) = bind_addr {
        start_peer_listener(main_tx.clone(), bind_addr);
    }
    let network = start_peer_senders(node_id.clone(), peers.clone());

    // Send lines from stdin to the main thread
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut stdin_lock = stdin.lock();
        let mut line = String::new();
        while stdin_lock
            .read_line(&mut line)
            .expect("error reading from stdin")
            != 0
        {
            let _ignore = main_tx.send(IncomingMessage::Append(line.clone().into()));
            line.clear();
        }
    });

    let mut raft = Node::new(
        node_id.clone(),
        peers.clone(),
        InMemoryLog::with_capacity(10240, RAFT_LOG_CAPACITY),
        OsRng::default(),
        RAFT_CONFIG,
    );

    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    let mut next_tick = Instant::now() + TICK_DURATION;
    loop {
        match main_rx.recv_timeout(next_tick.saturating_duration_since(Instant::now())) {
            Ok(IncomingMessage::Append(data)) => {
                // Append log entries from stdin
                match raft.append(data) {
                    Ok(new_messages) => new_messages.for_each(|message| network.send(message)),
                    Err(AppendError::Cancelled { data }) => {
                        log::info!("append cancelled: {}", String::from_utf8_lossy(&data))
                    }
                    Err(AppendError::LogErr(err)) => log::error!("raft log error: {:?}", err),
                }
            }
            Ok(IncomingMessage::Message(NetworkMessage { from, message })) => {
                // Process incoming message
                let new_messages =
                    raft.receive(message, String::from_utf8_lossy(&from).to_string());
                new_messages.for_each(|message| network.send(message));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Tick the timer
                let new_messages = raft.timer_tick();
                new_messages.for_each(|message| network.send(message));
                next_tick = Instant::now() + TICK_DURATION;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => panic!("child threads died"),
        }

        // Check for committed log entries
        for log_entry in raft.take_committed() {
            if !log_entry.data.is_empty() {
                stdout_lock
                    .write(&log_entry.data)
                    .expect("error writing to stdout");
            }
        }
    }
}

fn parse_args() -> Args {
    let mut args = std::env::args();
    let executable_name = args.next().unwrap_or_default();

    let (bind_addr, node_id) = match (args.next(), args.next()) {
        (Some(first_arg), _) if first_arg.starts_with('-') => usage(&executable_name),
        (Some(_), None) => usage(&executable_name),
        (Some(bind_addr), Some(node_id)) => (Some(bind_addr), node_id),
        (None, _) => (None, String::new()),
    };

    let peers = args.collect::<BTreeSet<_>>();

    Args {
        bind_addr,
        node_id,
        peers,
    }
}

fn usage(executable_name: &str) -> ! {
    eprint!(
        concat!(
            "Usage: {} [-h] [[bind_addr:]port node_host:port] [peer_host:port ...]\n",
            "\n",
            "[bind_addr:]port - the host:port to listen on\n",
            "node_host:port   - the public host:port of this node\n",
            "peer_host:port   - the public host:port of any peers\n",
        ),
        executable_name
    );
    std::process::exit(1)
}

fn start_peer_listener(main_tx: mpsc::Sender<IncomingMessage>, bind_addr: String) {
    let bind_addr = if bind_addr.contains(':') {
        bind_addr
    } else {
        format!("0.0.0.0:{}", bind_addr)
    };
    let listener = TcpListener::bind(&bind_addr)
        .unwrap_or_else(|error| panic!("error listening on {}: {}", bind_addr, error));
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            start_peer_receiver(
                BufReader::new(stream.expect("error accepting connecting")),
                main_tx.clone(),
            );
        }
    });
}

fn start_peer_receiver(mut reader: BufReader<TcpStream>, main_tx: mpsc::Sender<IncomingMessage>) {
    std::thread::spawn(move || {
        let addr = reader.get_mut().peer_addr().unwrap();
        log::info!("accepted connection from {}", addr);
        while let Ok(message) = read_peer_message(&mut reader)
            .map_err(|error| log::info!("error receiving from {}: {}", addr, error))
        {
            let _ignore = main_tx.send(IncomingMessage::Message(message));
        }
    });
}

fn read_peer_message(reader: &mut BufReader<TcpStream>) -> Result<NetworkMessage, Box<dyn Error>> {
    let mut len_data = [0; 4];
    reader.read_exact(&mut len_data)?;
    let mut message_data = vec![0; u32::from_be_bytes(len_data) as usize];
    reader.read_exact(&mut message_data)?;
    let message = NetworkMessage::decode(&message_data[..])
        .map_err(|error| format!("invalid message from peer: {}", error))?;
    log::debug!(
        "{} -> self: {}",
        String::from_utf8_lossy(&message.from),
        &message.message
    );
    Ok(message)
}

fn start_peer_senders(node_id: NodeId, peers: BTreeSet<NodeId>) -> Network {
    let (peers_tx, peers_rx): (BTreeMap<_, _>, Vec<_>) = peers
        .iter()
        .map(|peer_id| {
            let (peer_tx, peer_rx) = mpsc::channel();
            ((peer_id.clone(), peer_tx), (peer_id.clone(), peer_rx))
        })
        .unzip();

    for (peer_id, peer_rx) in peers_rx {
        start_peer_sender(node_id.clone().into(), peer_id, peer_rx);
    }

    Network { peers_tx }
}

fn start_peer_sender(from: Bytes, address: String, peer_rx: mpsc::Receiver<Message>) {
    std::thread::spawn(move || {
        let mut connection = None;
        let mut data = Vec::new();
        loop {
            let message =
                match peer_rx.recv_timeout(TICK_DURATION * RAFT_CONFIG.election_timeout_ticks) {
                    Ok(message) => Some(NetworkMessage {
                        from: from.clone(),
                        message,
                    }),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };

            if connection.is_none() {
                match TcpStream::connect(&address) {
                    Ok(established_connection) => {
                        log::info!("connected to {}", &address);
                        let _ignore = established_connection.set_nodelay(true);
                        connection = Some(established_connection);
                    }
                    Err(error) => log::info!("error connecting to {}: {}", &address, error),
                }
            }
            if let (Some(established_connection), Some(message)) = (&mut connection, &message) {
                data.clear();
                data.put_u32(message.encoded_len() as u32);
                message.encode(&mut data).unwrap();
                if let Err(error) = established_connection.write_all(&data) {
                    log::info!("error sending to {}: {}", &address, error);
                    connection = None;
                }
            }
        }
    });
}

impl Network {
    fn send(&self, sendable: SendableMessage<NodeId>) {
        match sendable.dest {
            MessageDestination::Broadcast => {
                log::debug!("self -> all: {}", sendable.message);
                self.peers_tx
                    .values()
                    .for_each(|peer_tx| drop(peer_tx.send(sendable.message.clone())));
            }
            MessageDestination::To(dst_id) => {
                log::debug!("self -> {}: {}", dst_id, sendable.message);
                let _ = self.peers_tx[&dst_id].send(sendable.message);
            }
        }
    }
}
