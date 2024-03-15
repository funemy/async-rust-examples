use futures::executor::block_on;
use async_example::counter::Counter;

async fn example() {
    let c = Counter::new(Some(1));
    c.await;
}

fn main() {
    block_on(example());
}
