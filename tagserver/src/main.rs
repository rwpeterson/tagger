use anyhow::Result;

use tagserver::{Config, controller, timer, server};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // read config file
    let mut cfg = Config::default();
    let args = std::env::args().collect::<Vec<_>>();
    if let Some(arg) = args.get(1) {
        if let Ok(addr) = arg.parse() {
            cfg = Config{ addr, ..cfg };
        }
    }

    let (tx_sync, rx_sync) = flume::unbounded();

    println!("starting timer");
    timer::main(cfg, tx_sync.clone())?;

    println!("starting controller");
    std::thread::spawn(move || -> Result<()> {
        controller::logic(cfg, rx_sync)?;
        Ok(())
    });

    println!("starting server");
    server::main(cfg, tx_sync.clone()).await?;

    Ok(())
}