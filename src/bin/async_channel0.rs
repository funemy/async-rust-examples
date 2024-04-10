use std::sync::atomic::Ordering::SeqCst;
use std::task::{Context, Poll};
use futures::channel::oneshot;
use futures::channel::oneshot::{Canceled, Receiver, Sender};
use smol::{block_on, Executor};

static EX: Executor<'_> = Executor::new();

fn sending(s : Sender<String>) {
    let res = s.send("1111111111111111111".to_owned());
    match res {
        Ok(_) => println!("data sent to channel"),
        Err(_) => println!("channel cancelled")
    }
}

async fn receiving(mut r : Receiver<String>) {
    r.close();
    let d = r.await;
    if let Ok(dd) = d {
        println!("data: {}", dd);
    } else {
        println!("channel cancelled");
    }
}

async fn example () {
    let (s, mut r) = oneshot::channel();
    let t = EX.spawn(receiving(r));
    sending(s);
    t.await
}

fn main() {
    block_on(EX.run(example()))
}

// High-level informal spec:
//  Assume `s.send` is concurrently called,
//  assume no cancellation for now,
//  check if the channel has completed, if so, take the data stored in the slot, and return Ready
//  if not, stored a handle of the task into rx_task field and return Pending.

// ready condition:
//  slot.take() == Some(data) /\
//  self.data.try_lock() == Some(mut slot) /\
//  done || self.complete.load(SeqCst)

// pending condition:
//  !done /\ !self.complete.load(SeqCst)

// fn recv(&self, cx: &mut Context<'_>) -> Poll<Result<T, Canceled>> {
//     let done = if self.complete.load(SeqCst) {
//         true
//     } else {
//         let task = cx.waker().clone();
//         match self.rx_task.try_lock() {
//             Some(mut slot) => {
//                 *slot = Some(task);
//                 false
//             }
//             None => true,
//         }
//     };
//
//     if done || self.complete.load(SeqCst) {
//        if let Some(data) = slot.take() {
//            return Poll::Ready(Ok(data));
//        }
//        unreachable!()
//     } else {
//         Poll::Pending
//     }
// }


// High-level informal spec:
//  If the channel is already complete, return Err
//  If the channel is not complete yet, store the data into the slot,
//  then check if rx_task is stored, if so, notify the task, if not return OK.

// fn send(&self, t: T) -> Result<(), T> {
//     if self.complete.load(SeqCst) {
//         return Err(t);
//     }
//
//     assert!(slot.is_none());
//     *slot = Some(t);
//     drop(slot);
//
//     Ok(())
// }
