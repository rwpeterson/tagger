use anyhow::Result;

pub mod client;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    println!("hello");
    let args = std::env::args().collect::<Vec<_>>();
    if let Some(s) = args.get(1) {
        if let Ok(sa) = s.parse::<std::net::SocketAddr>() {
            match client::main(sa).await {
                Ok(()) => {},
                Err(e) => {
                    println!("{}",e);
                    anyhow::bail!("oops");
                }
            }
        } else {
            let sa = "127.0.0.1:6969".parse::<std::net::SocketAddr>();
            client::main(sa.unwrap()).await.unwrap();
        }
    } else {
        let sa = "127.0.0.1:6969".parse::<std::net::SocketAddr>();
        client::main(sa.unwrap()).await.unwrap();
    }


    Ok(())
}