#![no_std]

extern crate alloc;

use core::cell::Cell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicU32, Ordering};
use core::task::{Context, Poll};

use crate::wake_list::{WakeHandle, WakeList};

mod wake_list;

/// A MPMC single-buffer channel for `Copy` types.
///
/// Writing is instant, receiving will always wait for a value from the future.
/// It is not guaranteed a waiting future will receive all messages - if a (second) new message is
/// written before a waiting future is polled, it will miss the first message.
struct Announcement<T> {
    message: Cell<Option<T>>,
    wake_list: WakeList,
    gen: AtomicU32,
}

impl<T> Announcement<T> {
    pub const fn new() -> Announcement<T> {
        Announcement {
            message: Cell::new(None),
            wake_list: WakeList::new(),
            gen: AtomicU32::new(u32::MIN),
        }
    }

    pub fn announce(&self, message: T) {
        self.gen.fetch_add(1, Ordering::Relaxed);
        self.message.set(Some(message));
        self.wake_list.wake_all();
    }
}

impl<T: Copy> Announcement<T> {
    pub async fn recv(&self) -> T {
        let fut = AnnouncementFut {
            ann: self,
            wh: None,
            my_gen: self.gen.load(Ordering::Relaxed) + 1,
        };

        fut.await
    }
}

struct AnnouncementFut<'a, T> {
    ann: &'a Announcement<T>,
    wh: Option<WakeHandle>,
    my_gen: u32,
}

impl<T: Copy> Future for AnnouncementFut<'_, T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.wh.is_none() {
            let mut wh = WakeHandle::new();
            wh.register(&self.ann.wake_list, cx.waker().clone());

            let _ = self.wh.insert(wh);
            Poll::Pending
        } else {
            if self.my_gen <= self.ann.gen.load(Ordering::Relaxed) {
                if let Some(message) = self.ann.message.get() {
                    return Poll::Ready(message);
                }
            }

            Poll::Pending
        }
    }
}