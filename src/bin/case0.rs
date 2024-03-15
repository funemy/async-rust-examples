use futures::executor::block_on;

// simplest async function, an "async" version of a sync function
async fn case0() -> () {
    println!("hello");
    println!("world");
    ()
}

// #[tokio::main]
fn main() {
    let simplest_future = case0();
    block_on(simplest_future)
}
