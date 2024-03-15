use std::{
    sync::atomic::AtomicBool,
    pin::Pin,
    future::Future,
    sync::atomic::Ordering::SeqCst,
    sync::Mutex,
    task::{Context, Poll, Waker},
    sync::Arc
};

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
            match self.data.lock() {
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
            Poll::Pending
        }
    }

    fn send(self: &Self, t: T) -> Result<(), T> {
        if self.complete.load(SeqCst) {
            return Err(t);
        }

        match self.data.lock() {
            Ok(mut data) => {
                assert!(data.is_none());
                *data = Some(t)
            }
            Err(_) => {
                unreachable!()
            }
        }
        Ok(())
    }
}