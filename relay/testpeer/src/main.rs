mod echo;

use echo::EchoPeer;

#[tokio::main]
async fn main() {
    // `--burst N` rather than a flag library: this binary takes one optional
    // argument and is spawned by a test, not by a person.
    let mut args = std::env::args().skip(1);
    let mut burst = None;
    let mut jukebox = None;
    let mut echo = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--burst" => burst = args.next().and_then(|value| value.parse().ok()),
            "--jukebox" => jukebox = args.next(),
            "--echo" => echo = true,
            _ => {}
        }
    }

    if let Err(e) = EchoPeer::run(burst, jukebox, echo).await {
        eprintln!("echo peer failed: {e}");
        std::process::exit(1);
    }
}
