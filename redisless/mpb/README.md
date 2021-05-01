# Multi Producer Broadcast

Crate for Multi-Producer Broadcast to do many to many (N*N) message passing.

```rust
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
```
