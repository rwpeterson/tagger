// Adapted from the capnproto-rust pubsub example code at
// https://github.com/capnproto/capnproto-rust/blob/master/capnp-rpc/examples/pubsub/client.rs
// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors

use capnp::capability::Promise;
use capnp_rpc::{RpcSystem, twoparty, rpc_twoparty_capnp, pry};
use futures::AsyncReadExt;
use tagger_capnp::tag_server_capnp::{publisher, subscriber, service_pub};
use tagtools::Tag;
use tokio::runtime::Builder;
use tokio::sync::oneshot;

struct Client {
    receiver: flume::Receiver<ClientMessage>,
}
enum ClientMessage {
    GetData {
        respond_to: oneshot::Sender<u8>,
    }
}

impl Client {
    fn new(receiver: flume::Receiver<ClientMessage>) -> Self {
        Client {
            receiver,
        }
    }
    fn handle_message(&mut self, msg: ClientMessage) {
        match msg {
            ClientMessage::GetData { respond_to } => {
                let _ = respond_to.send(69);
            },
        }
    }
}

async fn run_client(mut c: Client) { // TODO: this should call what is now main
    while let Ok(msg) = c.receiver.recv_async().await {
        c.handle_message(msg);
    }
}

#[derive(Clone)]
pub struct ClientHandle {
    sender: flume::Sender<ClientMessage>,
}

impl ClientHandle {
    pub fn new() -> Self {
        let (sender, receiver) = flume::bounded(1);
        let rpc_client = Client::new(receiver);
        let rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        std::thread::spawn(move || {
            rt.block_on(async move {
                run_client(rpc_client);
            });
        });

        ClientHandle {
            sender,
        }
    }
}

pub struct StreamData {
    pub tagpat: TagPattern,
    pub pats:   Vec<LogicPattern>,
}

pub struct TagPattern {
    pub tagmask:  u16,
    pub duration: u64,
    pub tags:     Vec<Tag>,
}

pub struct LogicPattern {
    pub patmask:  u16,
    pub duration: u64,
    pub count:    u64,
}

struct SubscriberImpl {
    sender: flume::Sender<StreamData>,
}

impl subscriber::Server<service_pub::Owned> for SubscriberImpl {
    fn push_message(
        &mut self,
        params: subscriber::PushMessageParams<service_pub::Owned>,
        _results: subscriber::PushMessageResults<service_pub::Owned>,
    ) -> Promise<(), ::capnp::Error> {
        let mut tags: Vec<Tag> = Vec::new();
        let mut tagmask = 0;
        let mut duration = 0;
        let mut pats: Vec<LogicPattern> = Vec::new();
        if pry!(pry!(params.get()).get_message()).has_tags() {
            let rdr = pry!(pry!(pry!(params.get()).get_message()).get_tags());
            tagmask = rdr.get_tagmask();
            duration = rdr.get_duration();
            let tags_rdr = pry!(rdr.get_tags());
            for chunk in pry!(tags_rdr.get_tags()).iter() {
                for tag in pry!(chunk).iter() {
                    tags.push(Tag { time: tag.get_time(), channel: tag.get_channel() } );
                }
            }
        }
        let tagpat = TagPattern {
            tagmask,
            duration,
            tags,
        };
        if pry!(pry!(params.get()).get_message()).has_pats() {
            for pat_rdr in pry!(pry!(pry!(params.get()).get_message()).get_pats()) {
                pats.push(LogicPattern {
                    patmask:  pat_rdr.get_patmask(),
                    duration: pat_rdr.get_duration(),
                    count:    pat_rdr.get_count(),
                });
            }
        }
        self.sender.send(StreamData {
            tagpat,
            pats,
        }).unwrap();
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
    let (sender, receiver) = flume::unbounded();

    tokio::task::LocalSet::new().run_until(async move {
        // Receives data from RPC calls and passes it to the app
        let client_future = async move {
            while let Ok(m) = receiver.recv_async().await {
                //lol
            }
            Ok(()) as capnp::Result<()>
        };

        // Manages the network connection and abstracts it into a Cap'n Proto RPC system
        let stream = tokio::net::TcpStream::connect(&addr).await?;
        stream.set_nodelay(true)?;
        let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
        let rpc_network =
            Box::new(twoparty::VatNetwork::new(
                reader,
                writer,
                rpc_twoparty_capnp::Side::Client,
                Default::default()),
            );
        let mut rpc_system = RpcSystem::new(rpc_network, None);

        let publisher: publisher::Client<service_pub::Owned> =
            rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
        let sub = capnp_rpc::new_client(SubscriberImpl { sender } );

        let mut request = publisher.subscribe_request();
        request.get().reborrow().set_subscriber(sub);
        let sbdr = request.get().init_services();
        let mut pbdr = sbdr.init_patmasks(1);
        pbdr.set(0, 2);

        // Need to make sure not to drop the returned subscription object.
        futures::future::try_join3(
            rpc_system,
            request.send().promise,
            client_future,
        ).await?;
        Ok(())
    }).await
}