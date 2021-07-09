// Adapted from the capnproto-rust pubsub example code at
// https://github.com/capnproto/capnproto-rust/blob/master/capnp-rpc/examples/pubsub/client.rs
// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors


use capnp_rpc::{RpcSystem, twoparty, rpc_twoparty_capnp, pry};
use crate::tag_server_capnp::{publisher, subscriber, service_pub};

use capnp::capability::Promise;
use futures::{AsyncReadExt};

use tagtools::Tag;

struct SubscriberImpl;

impl subscriber::Server<service_pub::Owned> for SubscriberImpl {
    fn push_message(
        &mut self,
        params: subscriber::PushMessageParams<service_pub::Owned>,
        _results: subscriber::PushMessageResults<service_pub::Owned>,
    ) -> Promise<(), ::capnp::Error> {
        let mut tags: Vec<Tag> = Vec::new();
        if pry!(pry!(params.get()).get_message()).has_tags() {
            let rdr = pry!(pry!(pry!(params.get()).get_message()).get_tags());
            let _tmask = rdr.get_tagmask();
            let _dur = rdr.get_duration();
            let tags_rdr = pry!(rdr.get_tags());
            for chunk in pry!(tags_rdr.get_tags()).iter() {
                for tag in pry!(chunk).iter() {
                    tags.push(Tag { time: tag.get_time(), channel: tag.get_channel() } );
                }
            }
            for tag in tags {
                println!(" T {0} Ch {1}", tag.time, tag.channel);
            }
        }
        if pry!(pry!(params.get()).get_message()).has_pats() {
            for lpat in pry!(pry!(pry!(params.get()).get_message()).get_pats()) {
                println!("pattern: {}", lpat.get_mask());
                println!(" -> counts: {}", lpat.get_count());
                println!(" -> duration: {}", lpat.get_duration());
            }
        }
        Promise::ok(())
    }
}

pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::net::ToSocketAddrs;
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {} client HOST:PORT", args[0]);
        return Ok(());
    }

    let addr = args[2].to_socket_addrs().unwrap().next().expect("could not parse address");

    tokio::task::LocalSet::new().run_until(async move {
        let stream = tokio::net::TcpStream::connect(&addr).await?;
        stream.set_nodelay(true)?;
        let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
        let rpc_network =
            Box::new(twoparty::VatNetwork::new(reader, writer,
                                               rpc_twoparty_capnp::Side::Client,
                                               Default::default()));
        let mut rpc_system = RpcSystem::new(rpc_network, None);
        let publisher: publisher::Client<service_pub::Owned> =
            rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
        let sub = capnp_rpc::new_client(SubscriberImpl);

        let mut request = publisher.subscribe_request();
        request.get().reborrow().set_subscriber(sub);
        let sbdr = request.get().init_services();
        let mut pbdr = sbdr.init_patmasks(1);
        pbdr.set(0, 2);

        // Need to make sure not to drop the returned subscription object.
        futures::future::try_join(rpc_system, request.send().promise).await?;
        Ok(())
    }).await
}