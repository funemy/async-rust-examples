use std::time::Duration;

use futures::executor::block_on;
use smol::Timer;

async fn case1() {
    println!("hello");
    println!("start waiting 6s!!");
    Timer::after(Duration::from_secs(6)).await;
    println!("done waiting 6s!!");
    println!("world");
}

// #[tokio::main]
fn main() {
    let future = case1();
    block_on(future)
}
