use std::str::from_utf8;
use futures::AsyncReadExt;
use smol::block_on;
use smol::fs::File;

async fn test() -> smol::io::Result<Vec<u8>> {
    let mut f = File::open("summary.txt").await?;
    let mut res = vec!();
    f.read_to_end(&mut res).await?;
    Ok(res)
}

fn main() {
    let r = block_on(test()).unwrap();
    println!("{}", from_utf8(&r).unwrap());
}