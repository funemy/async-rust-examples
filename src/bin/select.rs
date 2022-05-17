use futures::{future::FutureExt, pin_mut, select};

async fn task_one() {
    println!("doing task one");
}

async fn task_two() {
    println!("doing task two");
}

async fn race_tasks() {
    let t1 = task_one().fuse();
    let t2 = task_two().fuse();

    pin_mut!(t1, t2);

    select! {
        () = t1 => print!("task one completed first."),
        () = t2 => print!("task two completed first."),
    }
}

fn main() {
    futures::executor::block_on(race_tasks());
}
