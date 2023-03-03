# announcement

Extremely basic asynchronous single-buffer channel for synchronising `Copy` values among multiple futures.

Each message (or 'announcement') is broadcasted to every future waiting on it through the `Announcement::recv()` method.

Usage:
```rust
let acc = Announcement::new();

let f1 = acc.recv();
let f2 = acc.recv();
let f3 = async {
    acc.announce(123);
};

assert_eq!((123, 123, ()), join!(f1, f2, f3).await);
```
