use anyhow::Result;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{AsyncReadExt, FutureExt};
use std::sync::atomic::AtomicU64;
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::{Config, Event, TagServerImpl};
use crate::tag_server_capnp::tagger;

pub async fn main(
    cfg: Config,
    tx: flume::Sender<Event>,
) -> Result<()> {

    tokio::task::LocalSet::new()
        .run_until(async move {
            let listener = tokio::net::TcpListener::bind(&cfg.addr).await.unwrap();

            let tag_server_impl = TagServerImpl { cfg, tx, id: AtomicU64::new(1) };

            let tag_client: tagger::Client = capnp_rpc::new_client(tag_server_impl);

            loop {
                let (stream, _) = listener.accept().await.unwrap();
                stream.set_nodelay(true).unwrap();
                let (reader, writer) = TokioAsyncReadCompatExt::compat(stream).split();
                let network = twoparty::VatNetwork::new(
                    reader,
                    writer,
                    rpc_twoparty_capnp::Side::Server,
                    Default::default(),
                );

                let rpc_system =
                    RpcSystem::new(Box::new(network), Some(tag_client.clone().client));

                tokio::task::spawn_local(Box::pin(rpc_system.map(|_| ())));
            }   
        }).await;
    
    Ok(())

}