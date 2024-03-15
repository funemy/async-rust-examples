use futures::executor::block_on;
use smol::Executor;
use async_example::counter::Counter;

static EX: Executor<'_> = Executor::new();

async fn example() {
    let c = Counter::new(Some(1));

    let t = EX.spawn(c);
    t.await;
}

fn main() {
    block_on(EX.run(example()));
}
