use core::time;
use std::thread;

struct Song {
    name: String,
}

async fn learn_song() -> Song {
    println!("learning Never Gonna Give You Up");
    thread::sleep(time::Duration::from_secs(1));
    Song {
        name: "Never Gonna Give You Up".to_string(),
    }
}

async fn sing_song(song: Song) {
    println!("singing {}!", song.name)
}

async fn dance() {
    println!("dancing along with the song!");
    thread::sleep(time::Duration::from_secs(1));
    println!("finish dancing")
}

async fn learn_and_sing() {
    let song = learn_song().await;
    sing_song(song).await;
}

async fn async_main() {
    let f1 = learn_and_sing();
    let f2 = dance();

    // NOTE: `join!` only runs things concurrently but not parallelly
    // So `thread::sleep` will block all async tasks
    // The same behavior appears in `futures::join!`
    tokio::join!(f1, f2);
}

#[tokio::main]
async fn main() {
    async_main().await
}
