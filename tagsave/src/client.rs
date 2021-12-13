// Adapted from the capnproto-rust pubsub example code at
// https://github.com/capnproto/capnproto-rust/blob/master/capnp-rpc/examples/pubsub/client.rs
// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors

use anyhow::Result;
use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{AsyncReadExt, FutureExt};
use parking_lot::Mutex;
use std::sync::Arc;
use tagger_capnp::tag_server_capnp::{publisher, service_pub, subscriber};
use tagtools::{bit::chans_to_mask, cfg, Tag};
use tokio::runtime::Builder;
use tokio::sync::mpsc;

const WIN_DEFAULT: u32 = 1;

pub struct RawChannelState {
    pub invm: u16,
    pub dels: Vec<u32>,
    pub thrs: Vec<f64>,
}

struct Client {
    receiver: mpsc::UnboundedReceiver<ClientMessage>,
    buffer: Arc<Mutex<Vec<StreamData>>>,
    data_receiver: mpsc::UnboundedReceiver<StreamData>,
}

#[derive(Debug)]
pub enum ClientMessage {
    GetData {
        respond_to: flume::Sender<Option<Vec<StreamData>>>,
    },
    Shutdown,
}

pub enum ChannelSettings {
    ChannelInversion((u8, bool)),
    ChannelDelay((u8, u32)),
    ChannelThreshold((u8, f64))
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

pub struct ClientHandle {
    pub sender: mpsc::UnboundedSender<ClientMessage>,
    pub join_handle: std::thread::JoinHandle<Result<Box<RawChannelState>>>,
}

impl ClientHandle {
    pub fn new(addr: std::net::SocketAddr, config: cfg::Run) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let (data_sender, data_receiver) = mpsc::unbounded_channel();
        let mut rpc_client = Client::new(receiver, data_receiver);
        let rt = Builder::new_current_thread().enable_all().build().unwrap();

        let join_handle = std::thread::spawn(move || {
            // runtime is started here
            return rt.block_on(async move { rpc_client.main(addr, config, data_sender).await });
        });

        ClientHandle {
            sender,
            join_handle,
        }
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
    pub window: Option<u32>,
}

pub struct ChannelState {
    pub inversion_mask: u16,
    pub delays: Vec<u32>,
    pub thresholds: Vec<f64>,
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
                    window: match pat_rdr.get_window() {
                        0 => None,
                        w => Some(w),
                    },
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
    ) -> Result<Box<RawChannelState>> {
        tokio::task::LocalSet::new()
            .run_until(async move {
                // Receives data from RPC calls and passes it to the app
                let client_future = async move {
                    loop {
                        tokio::select! {
                            Some(msg) = self.receiver.recv() => {
                                match msg {
                                    ClientMessage::GetData { respond_to } => {
                                        let b = self.buffer.clone();
                                        let mut buffer = b.lock();
                                        let _ = match (*buffer).is_empty() {
                                            true => respond_to.send(None),
                                            false => {
                                                let data = (*buffer).drain(..).collect();
                                                respond_to.send(Some(data))
                                            },
                                        };
                                    }
                                    ClientMessage::Shutdown => break,
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
                let sub = capnp_rpc::new_client(SubscriberImpl {
                    sender: data_sender,
                });

                let _ = tokio::task::spawn_local(Box::pin(rpc_system.map(|_| ())));

                let mut pats = Vec::new();
                for s in config.clone().singles {
                    if let cfg::Single::Channel(ch) = s {
                        pats.push((chans_to_mask(&[ch]), None));
                    }
                }
                for c in config.clone().coincidences {
                    match c {
                        cfg::Coincidence::Channels((ch_a, ch_b)) => {
                            pats.push((chans_to_mask(&[ch_a, ch_b]), None));
                        }
                        cfg::Coincidence::ChannelsWin((ch_a, ch_b, win)) => {
                            let w = if win == 0 { None } else { Some(win) };
                            pats.push((chans_to_mask(&[ch_a, ch_b]), w));
                        }
                        cfg::Coincidence::ChannelsCounts(_) => {}
                    }
                }

                // Assemble the channel settings first
                let mut set_reqs = Vec::new();
                for cs in config.channel_settings.iter() {
                    let ch = cs.channel;
                    if let Some(del) = cs.delay {
                        let mut req = publisher.set_input_request();
                        let mut dbdr = req.get().init_s().init_delay();
                        dbdr.set_ch(ch);
                        dbdr.set_del(del);
                        set_reqs.push(req.send().promise);
                    }
                    if let Some(inv) = cs.invert {
                        let mut req = publisher.set_input_request();
                        let mut rbdr = req.get();
                        let mut dbdr = rbdr.reborrow().init_s().init_inversion();
                        dbdr.reborrow().set_ch(ch);
                        dbdr.reborrow().set_inv(inv);
                        set_reqs.push(req.send().promise);
                    }
                    if let Some(th) = cs.threshold {
                        let mut req = publisher.set_input_request();
                        let mut rbdr = req.get();
                        let mut dbdr = rbdr.reborrow().init_s().init_threshold();
                        dbdr.reborrow().set_ch(ch);
                        dbdr.reborrow().set_th(th);
                        set_reqs.push(req.send().promise);
                    }
                }
                // Run the channel settings futures to completion first, before requesting data
                futures::future::try_join_all(set_reqs).await?;

                // Assemble the service sub request
                let mut data_req = publisher.subscribe_request();
                data_req.get().reborrow().set_subscriber(sub);
                let sbdr = data_req.get().init_services();
                let mut pbdr = sbdr.init_patmasks().init_windowed(pats.len() as u32);
                for (i, (pat, win)) in pats.iter().enumerate() {
                    let mut lpbdr = pbdr.reborrow().get(i as u32);
                    lpbdr.reborrow().set_patmask(*pat);
                    lpbdr.reborrow().set_window(win.unwrap_or(WIN_DEFAULT));
                }

                // Assemble the channel settings get request
                let get_req = publisher.get_inputs_request();

                // Need to make sure not to drop the returned subscription object.
                let tuple = futures::future::try_join3(
                    data_req.send().promise,
                    get_req.send().promise,
                    client_future,
                );
                let (_, get_reply, _) = tuple.await?;
                let rdr = get_reply.get().unwrap().get_s().unwrap();
                let invm = rdr.reborrow().get_inversionmask();
                let dels: Vec<u32> = rdr.reborrow().get_delays().unwrap().iter().collect();
                let thrs: Vec<f64> = rdr.reborrow().get_thresholds().unwrap().iter().collect();
                let raw_settings = RawChannelState { invm, dels, thrs };
                Ok(Box::new(raw_settings))
            }
        ).await
    }
}