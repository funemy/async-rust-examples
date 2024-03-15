use std::time::Duration;

use futures::executor::block_on;
use smol::{Executor, Timer};

static EX: Executor<'_> = Executor::new();

async fn case2() {
    println!("hello");
    let t1 = wait2s();
    let t2 = wait4s();
    println!("6s!!");
    Timer::after(Duration::from_secs(6)).await;

    println!("done 6s!!");
    // this await eventually calls `poll_task` in async-task/task.rs
    t1.await;
    t2.await;
    println!("world");
}

async fn wait2s() {
    println!("start waiting for 2 sec.");
    Timer::after(Duration::from_secs(2)).await;
    println!("done waiting for 2 sec.");
}

async fn wait4s() {
    println!("start waiting for 4 sec.");
    Timer::after(Duration::from_secs(4)).await;
    println!("done waiting for 4 sec.");
}

// #[tokio::main]
fn main() {
    let future = case2();
    block_on(EX.run(future))
    // block_on(future)
}
