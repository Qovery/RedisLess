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
}

impl<X> MPB<X>
where
    X: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        let (sender, receiver) = unbounded::<X>();

        let mpb = MPB {
            sender,
            internal_senders: Arc::new(Mutex::new(vec![])),
        };

        mpb._init(receiver);

        mpb
    }

    fn _init(&self, receiver: Receiver<X>) {
        let internal_senders = self.internal_senders.clone();

        let _ = thread::spawn(move || {
            for msg in receiver {
                match internal_senders.lock() {
                    Ok(senders) => {
                        for sender in senders.iter() {
                            let _ = sender.send(msg.clone());
                        }
                    }
                    Err(_) => {} // TODO manage deadlock
                }
            }
        });
    }

    pub fn sender(&self) -> Sender<X> {
        self.sender.clone()
    }

    pub fn receiver(&self) -> Receiver<X> {
        let (sender, receiver) = unbounded();

        match self.internal_senders.lock() {
            Ok(mut s) => {
                s.push(sender);
            }
            Err(_) => {} // TODO manage deadlock
        }

        receiver
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use crate::MPB;

    #[test]
    fn test_1() {
        let mpb = MPB::new();

        let sender1 = mpb.sender();
        let sender2 = mpb.sender();

        let receiver1 = mpb.receiver();
        let receiver2 = mpb.receiver();

        let j1 = thread::spawn(move || {
            for receive in receiver1.recv() {
                assert_eq!(receive, "hello");
            }
        });

        let j2 = thread::spawn(move || {
            for receive in receiver2.recv() {
                assert_eq!(receive, "hello");
            }
        });

        let _ = sender1.send("hello");
        let _ = sender2.send("hello");

        let _ = j1.join();
        let _ = j2.join();
    }
}
