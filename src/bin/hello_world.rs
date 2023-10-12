// use futures::executor::block_on;

async fn hello_world() {
    println!("hello world");
}

#[tokio::main]
async fn main() {
    let future = hello_world();
    future.await
}
