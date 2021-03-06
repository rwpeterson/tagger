use std::io::Write;

use tagstream::{CliArgs, server};

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

    // Needed to make ansi escape codes work on Windows 10
    // Will likely panic on earlier versions of Windows
    enable_ansi_support::enable_ansi_support().unwrap();

    tracing_subscriber::fmt::init();

    server::main(args).await
}
