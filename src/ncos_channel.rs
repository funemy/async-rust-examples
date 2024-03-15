use std::{sync::atomic::AtomicBool, pin::Pin, future::Future, sync::atomic::Ordering::SeqCst, sync::Mutex, task::{Context, Poll, Waker}, sync::Arc, thread};
use std::time::Duration;

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner::new());
    let sender = Sender::new(inner.clone());
    let receiver = Receiver::new(inner);
    return (sender, receiver)
}

pub struct Receiver<T> {
    inner : Arc<Inner<T>>
}

pub struct Sender<T> {
    inner : Arc<Inner<T>>
}

// smol uses their custom lock instead of mutex
// The idea is that you don't want to call "lock"
// in an async library.
// Instead, you should always do "try_lock",
// therefore, they implemented their own lock
// which only exposes "try_lock".
struct Inner<T> {
    data : Mutex<Option<T>>,
    complete : AtomicBool,
    rx_task : Mutex<Option<Waker>>
}

impl <T> Inner<T> {
    pub fn new() -> Self {
        return Inner {
            data: Mutex::new(None),
            complete: false.into(),
            rx_task: Mutex::new(None),
        }
    }
}

impl <T> Receiver<T> {
    fn new(inner: Arc<Inner<T>>) -> Self {
        return Receiver { inner }
    }
}

impl <T> Sender<T> {
    fn new (inner: Arc<Inner<T>>) -> Self {
        return Sender { inner }
    }
}

impl <T> Future for Receiver<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.recv(cx)
    }
}

impl <T> Sender<T> {
    pub fn send(self: &Self, t: T) -> Result<(), T> {
        self.inner.send(t)
    }
}

// NOTE: The REALLY important stuff
impl <T>Inner<T> {
    fn recv(self: &Self, cx: &Context<'_>) -> Poll<T> {
        let done = if self.complete.load(SeqCst) {
            true
        } else {
            let task = cx.waker().clone();
            match self.rx_task.try_lock() {
                Ok(mut rx_task) => {
                    *rx_task = Some(task);
                    false
                }
                Err(_) => true
            }
        };

        if done || self.complete.load(SeqCst) {
            match self.data.try_lock() {
                Ok(mut d) => {
                    let data = d.take();
                    match data {
                        Some(data) => Poll::Ready(data),
                        None => unreachable!()
                    }
                }
                Err(_) => unreachable!()
            }
        } else {
            println!("receiver pending");
            Poll::Pending
        }
    }

    fn send(self: &Self, t: T) -> Result<(), T> {
        if self.complete.load(SeqCst) {
            return Err(t);
        }

        match self.data.try_lock() {
            Ok(mut data) => {
                assert!(data.is_none());
                *data = Some(t);
                // NOTE: setting complete here will be buggy
                // self.complete.store(true, SeqCst);
                thread::sleep(Duration::from_secs(3));
            }
            Err(_) => {
                unreachable!()
            }
        }
        // NOTE: I forgot the line below on my first try
        self.complete.store(true, SeqCst);

        // NOTE: and I forgot the code below on my second try
        match self.rx_task.try_lock() {
            Ok(mut rx_task) => {
                let rx = rx_task.take();
                if let Some(w) = rx {
                    w.wake()
                }
            }
            Err(_) => unreachable!()
        }

        Ok(())
    }
}