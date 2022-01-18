use std::io::Write;

use streamer::{CliArgs, server};

const GIT_VERSION: &str = git_version::git_version!();

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
     
    let args: CliArgs = argh::from_env();

    if args.version {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        writeln!(
            stdout,
            concat!(
                env!("CARGO_BIN_NAME"),
                " ",
                "{}",
            ),
            GIT_VERSION,
        )?;
        return Ok(())
    }

    // ansi_term (used by tracing for output) requires explicitly-enabled ANSI support on Windows
    // https://github.com/tokio-rs/tracing/issues/445
    // https://github.com/ogham/rust-ansi-term#basic-usage
    if cfg!(windows) {
        ansi_term::enable_ansi_support().unwrap();
    }

    tracing_subscriber::fmt::init();

    server::main(args).await
}
