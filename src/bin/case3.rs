use std::{
    io::Error,
    os::{fd::AsRawFd, unix::net::UnixStream},
};

use futures::executor::block_on;
use smol::{io::AsyncReadExt, Async, Executor};

static EX: Executor<'_> = Executor::new();

async fn case3() -> Result<(), Error> {
    println!("hello");
    let (a, mut b) = Async::<UnixStream>::pair().unwrap();

    signal_hook::low_level::pipe::register_raw(signal_hook::consts::SIGINT, a.as_raw_fd())?;
    println!("Waiting for Ctrl-C...");

    let buf = &mut [0];
    // Receive a byte that indicates the Ctrl-C signal occurred.
    b.read_exact(buf).await?;

    println!("world");
    Ok(())
}

// #[tokio::main]
fn main() {
    let future = case3();
    block_on(EX.run(future)).unwrap();
    // block_on(future)
}
