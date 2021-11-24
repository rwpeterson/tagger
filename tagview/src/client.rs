// Adapted from the capnproto-rust pubsub example code at
// https://github.com/capnproto/capnproto-rust/blob/master/capnp-rpc/examples/pubsub/client.rs
// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors

use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{AsyncReadExt, FutureExt};
use parking_lot::Mutex;
use std::sync::Arc;
use tagger_capnp::tag_server_capnp::{publisher, service_pub, subscriber};
use tagtools::{bit::chans_to_mask, Tag, cfg};
use tokio::runtime::Builder;
use tokio::sync::mpsc;

struct Client {
    receiver: mpsc::UnboundedReceiver<ClientMessage>,
    buffer: Arc<Mutex<Vec<StreamData>>>,
    data_receiver: mpsc::UnboundedReceiver<StreamData>,
}
pub enum ClientMessage {
    GetData {
        respond_to: flume::Sender<Option<Vec<StreamData>>>,
    },
}

impl Client {
    fn new(
        receiver: mpsc::UnboundedReceiver<ClientMessage>,
        data_receiver: mpsc::UnboundedReceiver<StreamData>,
    ) -> Self {
        Client {
            receiver,
            buffer: Arc::new(Mutex::new(Vec::new())),
            data_receiver,
        }
    }
}

#[derive(Clone)]
pub struct ClientHandle {
    pub sender: mpsc::UnboundedSender<ClientMessage>,
}

impl ClientHandle {
    pub fn new(addr: std::net::SocketAddr, config: cfg::Run) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let (data_sender, data_receiver) = mpsc::unbounded_channel();
        let mut rpc_client = Client::new(receiver, data_receiver);
        let rt = Builder::new_current_thread().enable_all().build().unwrap();

        std::thread::spawn(move || {
            rt.block_on(async move {
                rpc_client.main(addr, config, data_sender).await.unwrap();
            });
        });

        ClientHandle { sender }
    }
}

pub struct StreamData {
    pub tagpat: TagPattern,
    pub pats: Vec<LogicPattern>,
}

pub struct TagPattern {
    pub tagmask: u16,
    pub duration: u64,
    pub tags: Vec<Tag>,
}

pub struct LogicPattern {
    pub patmask: u16,
    pub duration: u64,
    pub count: u64,
}

struct SubscriberImpl {
    sender: mpsc::UnboundedSender<StreamData>,
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
                    tags.push(Tag {
                        time: tag.get_time(),
                        channel: tag.get_channel(),
                    });
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
                    patmask: pat_rdr.get_patmask(),
                    duration: pat_rdr.get_duration(),
                    count: pat_rdr.get_count(),
                });
            }
        }
        let _ = self.sender.send(StreamData { tagpat, pats });
        Promise::ok(())
    }
}

impl Client {
    async fn main(
        &mut self,
        addr: std::net::SocketAddr,
        config: cfg::Run,
        data_sender: mpsc::UnboundedSender<StreamData>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        tokio::task::LocalSet::new()
            .run_until(async move {
                // Receives data from RPC calls and passes it to the app
                let client_future = async {
                    loop {
                        tokio::select! {
                            Some(msg) = self.receiver.recv() => {
                                match msg {
                                    ClientMessage::GetData { respond_to } => {
                                        let b = &mut self.buffer;
                                        let mut buffer = b.lock();
                                        let _ = match (*buffer).is_empty() {
                                            true => respond_to.send(None),
                                            false => {
                                                let data = (*buffer).drain(..).collect();
                                                respond_to.send(Some(data))
                                            },
                                        };
                                    }
                                }
                            },
                            Some(msg) = self.data_receiver.recv() => {
                                let b = self.buffer.clone();
                                let mut buffer = b.lock();
                                (*buffer).push(msg);
                            },
                            else => break,
                        }
                    }
                    Ok(()) as capnp::Result<()>
                };

                // Manages the network connection and abstracts it into a Cap'n Proto RPC system
                let stream = tokio::net::TcpStream::connect(&addr).await?;
                stream.set_nodelay(true)?;
                let (reader, writer) =
                    tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
                let rpc_network = Box::new(twoparty::VatNetwork::new(
                    reader,
                    writer,
                    rpc_twoparty_capnp::Side::Client,
                    Default::default(),
                ));
                let mut rpc_system = RpcSystem::new(rpc_network, None);

                let publisher: publisher::Client<service_pub::Owned> =
                    rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
                let sub = capnp_rpc::new_client(SubscriberImpl { sender: data_sender });

                let _ = tokio::task::spawn_local(Box::pin(rpc_system.map(|_| ())));

                let mut pats: Vec<(u16, u32)> = Vec::new();
                for s in config.singles {

                    match s {
                        cfg::Single::Channel(ch) => {
                            pats.push((chans_to_mask(&[ch]), 0));
                        },
                        // Ignore recorded data
                        cfg::Single::ChannelCounts(_) => {},
                    }
                }
                for c in config.coincidences {
                    match c {
                        cfg::Coincidence::Channels((ch_a, ch_b)) => {
                            pats.push((chans_to_mask(&[ch_a, ch_b]), 0));
                        },
                        cfg::Coincidence::ChannelsWin((ch_a, ch_b, win)) => {
                            pats.push((chans_to_mask(&[ch_a, ch_b]), win));
                        },
                        // Ignore recorded data
                        cfg::Coincidence::ChannelsCounts(_) => {},
                    }
                }
                
                // Assemble the request
                let mut request = publisher.subscribe_request();
                request.get().reborrow().set_subscriber(sub);
                let sbdr = request.get().init_services();
                let mut pbdr = sbdr.init_patmasks().init_windowed(pats.len() as u32);
                for (i, &(pat, win)) in pats.iter().enumerate() {
                    let mut lpbdr = pbdr.reborrow().get(i as u32);
                    lpbdr.set_patmask(pat);
                    lpbdr.set_window(win);
                }
                let request_future = request.send().promise;

                // Need to make sure not to drop the returned subscription object.
                futures::future::try_join(
                    request_future,
                    client_future,
                ).await?;
                Ok(())
            }
        ).await
    }
}
