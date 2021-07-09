pub fn main(period: std::time::Duration, tx: flume::Sender<()>) -> anyhow::Result<()> {
    std::thread::spawn(move || {
        while let Ok(()) = tx.send(()) {
            std::thread::sleep(period)
        }
    });
    Ok(())
}