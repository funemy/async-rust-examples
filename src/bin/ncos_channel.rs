use std::thread;
use smol::{block_on, Executor};
use async_example::ncos_channel;
use async_example::ncos_channel::{Receiver, Sender};

static EX: Executor<'_> = Executor::new();

fn sending(s : Sender<String>) {
    let _ = s.send("1111111111111111111".to_owned());
    println!("data sent to channel")
}

async fn receiving(r : Receiver<String>) {
    // every future can only be awaited once
    // so we don't need to worry about r.await being called again
    let d = r.await;
    println!("data: {}", d);
}

async fn example () {
    let (s, r) = ncos_channel::channel();
    let t = EX.spawn(receiving(r));
    thread::spawn(|| {
        sending(s);
    });
    t.await
}

fn main() {
    block_on(EX.run(example()))
}
