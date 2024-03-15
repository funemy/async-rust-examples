struct Waker {}

impl Waker {
    fn wake(&self) {}
}

fn process_wakers(wakers: &mut Vec<Waker>)  {
    for waker in wakers {
        waker.wake()
    }
}

fn main() {
    let mut wakers = Vec::new();
    process_wakers(&mut wakers)
}