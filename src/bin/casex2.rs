use futures::executor::block_on;
use async_example::counter::Counter;

fn main() {
    let c = Counter::new(Some(1));
    block_on(c);
}
