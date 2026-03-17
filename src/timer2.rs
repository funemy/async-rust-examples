#![allow(dead_code)]
// #![feature(register_tool)]
// #![register_tool(raven)]
// #![feature(stmt_expr_attributes)]

use crate::{repo::{RVec, Repo}, jackdaw_spec::*};
use futures::{
    future::{BoxFuture, FutureExt},
    task::{waker_ref, ArcWake},
};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, Mutex, OnceLock,
    },
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

use jackdaw_macros::*;

// Timer interface
// Timer has a shared state for communication between the main thread and the timer thread
pub struct Timer {
    instant: Instant,
}

// The state (i.e., responsibility) sent from a Timer to a Reactor.
#[derive(Clone, Debug)]
struct Shared {
    instant: Instant,
    waker: Waker,
}

impl Future for Timer {
    // No return value when the timer finishes
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.instant < Instant::now() {
            Poll::Ready(())
        } else {
            let shared = Shared {
                instant: self.instant,
                waker: cx.waker().clone(),
            };
            Reactor::insert(shared);
            Poll::Pending
        }
    }
}

impl Timer {
    // Constructor for Timer
    pub fn new(duration: Duration) -> Self {
        let instant = Instant::now() + duration;
        Timer { instant }
    }
}

// Executor for Timer
// The task queue of the executor is modeled by a channel.
// When scheduling a task, the `spawn` function send the given task into the channel
// (therefore the task stays in the buffer of the channel)
struct Executor {
    // Tasks are sent through a channel
    ready_queue: Receiver<Arc<Task>>,

    task_sender: Option<SyncSender<Arc<Task>>>,
}

struct Reactor {
    wakers: Arc<Mutex<RVec<Shared>>>,
}

impl Reactor {
    fn new() -> Self {
        Self {
            wakers: Arc::new(Mutex::new(RVec::default())),
        }
    }

    fn get() -> &'static Self {
        // It is worth noting that, in smol, the global reactor is initialized using a customized
        // async version of OnceCell, while I used a sync'd version (i.e., `OnceLock`) provided by
        // the std.
        // That means, the implementation of the smol async runtime, is actually dependent on a
        // much smaller async runtime, i.e., the async OnceCell, which further rely on another
        // crate called `event_listener` (the project is under the smol github org).
        // AFAICT, `event_listener` is a more lightweight async runtime that at least support
        // primitives like `OnceCell`.
        static REACTOR: OnceLock<Reactor> = OnceLock::new();
        REACTOR.get_or_init(|| Reactor::new())
    }

    // repo: timer_wakers: Timer
    #[raven::reactor(TimerReactor)]
    #[raven::repo_decl(timers: Timer)]
    fn react(&self) {
        inf_loop! {
            let mut wakers = self.wakers.lock().unwrap();

            repo![ @partition: timers -> (ready, pending) | e1 ];
            let (ready, pending) = wakers.clone().partition(|s| s.instant < Instant::now());

            repo![ @wake: ready ];
            for s in ready {
                s.waker.wake();
            }

            repo![ @merge: timers <- pending ];
            *wakers = pending;
        }
    }

    fn inner_insert(&self, s: Shared) {
        self.wakers.lock().unwrap().insert(s);
    }

    fn insert(s: Shared) {
        Reactor::get().inner_insert(s);
    }
}

struct Task {
    future: Mutex<Option<BoxFuture<'static, ()>>>,

    // This part is tricky and serves two purposes:
    // 1. to implement the `wake` function, we need a way to "put the task back into the queue", and `task_sender` is our handle of the queue
    // 2. `task_sender` drops when the task drops, and the channel closes when all of its sender drops. Moreover, the Executor exits when the channel is closed,
    //    so this field also act as a signal for Executor to exit.
    task_sender: SyncSender<Arc<Task>>,
}

impl ArcWake for Task {
    // `wake` function
    // send the task back to the task queue when it's ready to make further progresses.
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let cloned = arc_self.clone();
        arc_self
            .task_sender
            .send(cloned)
            .expect("too many tasks queued.")
    }
}

// NOTE: Executor Actor
impl Executor {
    // constructor
    fn new() -> Self {
        const MAX_QUEUE_SIZE: usize = 10_000;
        let (task_sender, ready_queue) = sync_channel(MAX_QUEUE_SIZE);
        Executor {
            ready_queue,
            task_sender: Some(task_sender),
        }
    }

    fn run(mut self) {
        self.task_sender.take();
        // Receive the next pending task from the ready queue
        while let Ok(task) = self.ready_queue.recv() {
            let mut future_slot = task.future.lock().unwrap();
            if let Some(mut future) = future_slot.take() {
                // make a waker instance from the task
                // remember that task implements ArcWake trait
                let waker = waker_ref(&task);
                // make a context instance from the waker
                let cx = &mut Context::from_waker(&waker);
                // poll the future with the context
                if future.as_mut().poll(cx).is_pending() {
                    *future_slot = Some(future)
                }
            }
        }
    }

    // An interface for spwaning tasks
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        // Make Rust's type system happy
        let future = future.boxed();
        // Create the task instance
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            task_sender: self.task_sender.clone().unwrap().clone(),
        });
        // Send the task through the channel
        self.task_sender
            .clone()
            .unwrap()
            .send(task)
            .expect("too many tasks queued.");
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::{executor::block_on, future::join};

    #[test]
    fn timer_test() {
        // we could also use a lazy-static variable to spawn this background thread implicitly
        let _ = std::thread::spawn(|| {
            Reactor::get().react();
        });

        let executor = Executor::new();
        executor.spawn(async {
            println!("Hello1");
            Timer::new(Duration::from_secs(2)).await;
            println!("Done1");
        });
        executor.spawn(async {
            println!("Hello2");
            Timer::new(Duration::from_secs(3)).await;
            println!("Done2");
        });
        // The reason we need a `block(join(...))` here is because `executor.run` is a blocking
        // call. In order for async tasks managed by the executor to run in parallel with the main
        // function (i.e., the other half of the `join`), we could either fork a thread for the
        // executor (then sharing the executor with the main function becomes a problem) or using
        // the `join` combinator at the end so the main function and the executor could run in
        // parallel.
        block_on(join(
            async {
                println!("Hello3");
                Timer::new(Duration::from_secs(5)).await;
                println!("Done3");
            },
            async { executor.run() },
        ));
    }
}
