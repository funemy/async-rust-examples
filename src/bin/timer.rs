use futures::{
    future::{BoxFuture, FutureExt},
    task::{waker_ref, ArcWake},
};
use std::{
    future::Future,
    pin::Pin,
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};

// Timer interface
// Timer has a shared state for communication between the main thread and the timer thread
pub struct Timer {
    shared_state: Arc<Mutex<SharedState>>,
}

// Timer's shared state
// completed: the flag indicating whether the timer has completed
// waker: the callback to invoke when timer finishes, so the Executor can poll again on this future (i.e. Timer)
struct SharedState {
    completed: bool,
    waker: Option<Waker>,
}

// Prop of Futures/Tasks
// forall T : Task . exist deps : Seq[Task] .
//  forall i j : Int . 0 <= i < j <= len(deps) . !done(deps(i)) => !done(deps(j)) & awaited
//  & (forall t in deps . done(t)) ~> done(T)
// TODO: 2. awaited(t) -- whether a task is being awaited, i.e., calling `t.await`
// NOTE: Future Actor
impl Future for Timer {
    // No return value when the timer finishes
    type Output = ();

    // The Executor provides a waker implementation in `cx`
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // get a mutable ref to the shared_state
        let mut state = self.shared_state.lock().unwrap();
        if state.completed {
            // if the timer has finished
            // return `Ready`, indicating the future can return from awaiting
            Poll::Ready(())
        } else {
            // if not
            // pass the waker to the shared state (the waker is inited to be `None`, check the constructor of `Timer`)
            // return `Pending`, indicating the future hasn't finish and will call waker later
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

// NOTE: Timer Actor
impl Timer {
    // Constructor for Timer
    pub fn new(duration: Duration) -> Self {
        // Initialize the shared state
        let shared_state = Arc::new(Mutex::new(SharedState {
            completed: false,
            waker: None,
        }));

        // Make a thread-safe ref to the share state, passing to the timer thread
        let thread_shared_state = shared_state.clone();

        // Timer thread
        // Transfer the ownership of `thread_shared_state` to the timer thread
        // Sleep for the given duration
        // After the sleep, set the `completed` flag to true
        // If the waker has been set by the executor, then invoke `waker.wake()` (asking the Executor to poll again)
        thread::spawn(move || {
            thread::sleep(duration);
            println!("timeout!");
            let mut shared_state = thread_shared_state.lock().unwrap();
            shared_state.completed = true;
            if let Some(waker) = shared_state.waker.take() {
                waker.wake();
            }
        });

        // return the timer future handle
        Timer { shared_state }
    }
}

// Executor for Timer
// The task queue of the executor is modelled by a channel.
// When scheduling a task, the `spawn` function send the given task into the channel
// (therefore the task stays in the buffer of the channel)
struct Executor {
    // Tasks are sent through a channel
    ready_queue: Receiver<Arc<Task>>,

    task_sender: Option<SyncSender<Arc<Task>>>,
}

struct Task {
    future: Mutex<Option<BoxFuture<'static, ()>>>,

    // This part is tricky and serves two purposes:
    // 1. to implement the `wake` function, we need a way to "put the task back into the queue", and `task_sender` is our handle of the queue
    // 2. `task_sender` drops when the task drops, and the channel closes when all of its sender drops. Moreover, the Executor exits when the channel is closed,
    //    so this field also act as a signal for Executor to exit.
    task_sender: SyncSender<Arc<Task>>,
}

// NOTE: Waker Actor
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

    // Prop 1 (Executor::run)
    // forall t : Task . ex : Executor .
    //  scheduled(t) ~> t.poll(cx)
    //      where cx : Context . cx.wake() & awaited(t) ~> t.poll(cx)
    //
    // Intuitively, the property above captures the essential behavior of Executor::run for guaranteeing task responsiveness.
    // To verify this, we need to futher define the two conditions:
    // 1. scheduled(t) -- t is in the active queue of the executor.
    //    To concretely define this, we need to specify what is the "active queue" and what it means for a task to "be in the active queue".
    //
    // This naive Executor handles tasks one-by-one from the task queue.
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
                    // NOTE: I'm slightly confused by this
                    // We haven't finished processing the future yet
                    // So put this future back to the futuer slot, otherwise the task will carry an empty future?
                    *future_slot = Some(future)
                }
            }
        }
    }

    // Prop 2 (Executor::spawn)
    // forall t : Task .
    //  spawn(t) ~> scheuled(t)
    //
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

// NOTE:
// The reason this main function terminates is very tricky
// Executor:run is in a while-loop, conditioned on `recv`
// `recv` only returns when all senders are dropped.
// The components that carry a sender are Executor and all tasks.
// At the start of `Executor::run`, executor drops its sender.
// Therefore, the channel will be closed when all tasks are dropped,
// and then the while-loop will exit and `run`A will return.
// NOTE:
// However, this should not affect our proof, since we only want to prove:
//   whether executor guarantees the responsivenesss of task
// But not the responsiveness of executor itself.
fn main() {
    let executor = Executor::new();
    executor.spawn(async {
        println!("Hello");
        Timer::new(Duration::from_secs(2)).await;
        println!("Done");
    });
    executor.run()
}
