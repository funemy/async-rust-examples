use std::{
    future::Future,
    pin::Pin,
    sync::atomic::AtomicBool,
    sync::atomic::Ordering::SeqCst,
    sync::Arc,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

use jackdaw_macros::*;

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner::new());
    let sender = Sender::new(inner.clone());
    let receiver = Receiver::new(inner);
    return (sender, receiver);
}

pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
}

// smol uses their custom lock instead of mutex
// The idea is that you don't want to call "lock"
// in an async library.
// Instead, you should always do "try_lock",
// therefore, they implemented their own lock
// which only exposes "try_lock".
struct Inner<T> {
    data: Mutex<Option<T>>,
    complete: AtomicBool,
    rx_task: Mutex<Option<Waker>>,
}

impl<T> Inner<T> {
    pub fn new() -> Self {
        return Inner {
            data: Mutex::new(None),
            complete: false.into(),
            rx_task: Mutex::new(None),
        };
    }
}

impl<T> Receiver<T> {
    fn new(inner: Arc<Inner<T>>) -> Self {
        return Receiver { inner };
    }
}

impl<T> Sender<T> {
    fn new(inner: Arc<Inner<T>>) -> Self {
        return Sender { inner };
    }
}

impl<T> Future for Receiver<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.recv(cx)
    }
}

impl<T> Sender<T> {
    pub fn send(self: &Self, t: T) -> Result<(), T> {
        self.inner.send(t)
    }
}

event_decl!(e1, "self.complete is set to true / channel is consumed");
event_decl!(e2, "self.rx_task is (first) set to the current task");

// NOTE: The REALLY important stuff
impl<T> Inner<T> {
    #[raven::pollable(NCOSChannel)]
    #[raven::resp_transfer( e2 <: e1 )]
    #[raven::complete(e1)]
    fn recv(&self, cx: &Context<'_>) -> Poll<T> {
        let done = if self.complete.load(SeqCst) {
            e1_tt!();
            true
        } else {
            e1_ff!();
            let task = cx.waker().clone();
            match self.rx_task.try_lock() {
                Ok(mut rx_task) => {
                    *rx_task = Some(task);
                    e2_tt!();
                    // NOTE: this can trigger "send@2"
                    // thread::sleep(Duration::from_secs(3));
                    false
                }
                Err(_) => {
                    e1_tt!();
                    true
                }
            }
        };

        if done || self.complete.load(SeqCst) {
            e1_tt!();
            match self.data.try_lock() {
                Ok(mut d) => {
                    let data = d.take();
                    match data {
                        Some(data) => Poll::Ready(data),
                        // NOTE: it's technically possible for recv to be wake up after it's finished,
                        //  but this is prevented by wake function, because task has an internal
                        //  state to indicate its completion
                        None => unreachable!("recv@1"),
                    }
                }
                Err(_) => unreachable!("recv@2"),
            }
        } else {
            e1_ff!();
            println!("receiver pending");
            Poll::Pending
        }
    }

    #[raven::reactor(NCOSChannelSend)]
    #[raven::singleton_decl(ch: NCOSChannel)]
    #[raven::terminate(e1)]
    fn send(&self, t: T) -> Result<(), T> {
        // prevent re-sending
        if self.complete.load(SeqCst) {
            e1_tt!();
            return Err(t);
        }
        e1_ff!();

        match self.data.try_lock() {
            Ok(mut data) => {
                assert!(data.is_none());
                *data = Some(t);
            }
            Err(_) => {
                unreachable!("send@1")
            }
        }
        self.complete.store(true, SeqCst);
        e1_tt!();

        match self.rx_task.try_lock() {
            Ok(mut rx_task) => {
                let rx = rx_task.take();
                if let Some(w) = rx {
                    e2_tt!();
                    println!("wake up rx_task");
                    repo![ @wake: ch ];
                    w.wake();
                }
                e2_ff!();
            }
            Err(_) => unreachable!("send@2"),
        }
        e2_ff!();
        Ok(())
    }
}
