use streamer::server;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _args: Vec<String> = ::std::env::args().collect();
    server::main().await
}