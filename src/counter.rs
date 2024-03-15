use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::task::Poll::{Pending, Ready};

pub struct Counter {
    counts: u32,
}

impl Counter {
    pub fn new(counts: Option<u32>) -> Self {
        match counts {
            None => Counter {counts: 0},
            Some(c) => {
                Counter {counts: c}
            }
        }
    }
}

// This implementation is essentially a more generalized implementation
// of `yield_now`
impl Future for Counter {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.counts == 0 {
            println!("counts is 0, done");
            Ready(())
        } else {
            println!("counts is {}, pending", self.counts);
            self.counts -= 1;
            cx.waker().clone().wake();
            Pending
        }
    }
}