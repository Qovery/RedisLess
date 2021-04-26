use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{unbounded, Receiver, Sender};

/// Multi-Producer Broadcast to do many to many (N*N) message passing.
pub struct MPB<X>
where
    X: Clone + Send + Sync + 'static,
{
    sender: Sender<X>,
    internal_senders: Arc<Mutex<Vec<Sender<X>>>>,
    receivers: Vec<Receiver<X>>,
}

impl<X> MPB<X>
where
    X: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        let (tx, rx) = unbounded::<X>();

        let mpb = MPB {
            sender: tx,
            internal_senders: Arc::new(Mutex::new(vec![])),
            receivers: vec![],
        };

        mpb._init(rx);

        mpb
    }

    fn _init(&self, rx: Receiver<X>) {
        let internal_senders = self.internal_senders.clone();

        let _ = thread::spawn(move || {
            let senders = internal_senders;

            for msg in rx.recv() {
                match senders.lock() {
                    Ok(s) => {
                        for sender in s.iter() {
                            let _ = sender.send(msg.clone());
                        }
                    }
                    Err(_) => {} // TODO manage deadlock
                }
            }
        });
    }

    pub fn tx(&self) -> Sender<X> {
        self.sender.clone()
    }

    pub fn rx(&mut self) -> Receiver<X> {
        let (_tx, rx) = unbounded();

        match self.internal_senders.lock() {
            Ok(mut s) => {
                s.push(_tx);
            }
            _ => {} // TODO manage deadlock
        }

        rx
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use crate::MPB;

    #[test]
    fn test_1() {
        let mut mpb = MPB::new();

        let tx1 = mpb.tx();
        let tx2 = mpb.tx();

        let rx1 = mpb.rx();
        let rx2 = mpb.rx();

        let j1 = thread::spawn(move || {
            for rx in rx1.recv() {
                assert_eq!(rx, "hello");
            }
        });

        let j2 = thread::spawn(move || {
            for rx in rx2.recv() {
                assert_eq!(rx, "hello");
            }
        });

        let _ = tx1.send("hello");
        let _ = tx2.send("hello");

        let _ = j1.join();
        let _ = j2.join();
    }
}
