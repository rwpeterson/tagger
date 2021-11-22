use streamer::{CliArgs, server};

const GIT_VERSION: &str = git_version::git_version!();

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
     
    let args: CliArgs = argh::from_env();

    if args.version {
        println!(
            concat!(
                env!("CARGO_BIN_NAME"),
                " ",
                "{}",
            ),
            GIT_VERSION,
        );
        return Ok(())
    }

    tracing_subscriber::fmt::init();

    server::main(args).await
}