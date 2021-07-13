// Adapted from the capnproto-rust pubsub example code at
// https://github.com/capnproto/capnproto-rust/blob/master/capnp-rpc/examples/pubsub/server.rs
// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors

use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{AsyncReadExt, FutureExt, StreamExt};
use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::tag_server_capnp::{publisher, service_pub, subscriber, subscription};

use crate::data::PubData;

use crate::Event;

pub struct SubscriberHandle {
    client: subscriber::Client<::capnp::any_pointer::Owned>,
    requests_in_flight: i32,
    tagmask: u16,
    patmasks: Vec<u16>,
}

pub struct SubscriberMap {
    subscribers: HashMap<u64, SubscriberHandle>,
}

impl SubscriberMap {
    fn new() -> SubscriberMap {
        SubscriberMap {
            subscribers: HashMap::new(),
        }
    }
}

struct SubscriptionImpl {
    id: u64,
    subscribers: Arc<Mutex<SubscriberMap>>,
}

impl SubscriptionImpl {
    fn new(id: u64, subscribers: Arc<Mutex<SubscriberMap>>) -> SubscriptionImpl {
        SubscriptionImpl {
            id: id,
            subscribers: subscribers,
        }
    }
}

impl Drop for SubscriptionImpl {
    fn drop(&mut self) {
        println!("subscription dropped");
        self.subscribers.lock().subscribers.remove(&self.id);
    }
}

impl subscription::Server for SubscriptionImpl {}

struct PublisherImpl {
    next_id: u64,
    subscribers: Arc<Mutex<SubscriberMap>>,
    cur_tagmask: Arc<RwLock<u16>>,
    cur_patmasks: Arc<RwLock<HashSet<u16>>>,
}

impl PublisherImpl {
    pub fn new() -> (
        PublisherImpl,
        Arc<Mutex<SubscriberMap>>,
        Arc<RwLock<u16>>,
        Arc<RwLock<HashSet<u16>>>,
    ) {
        let subscribers = Arc::new(Mutex::new(SubscriberMap::new()));
        let cur_tagmask = Arc::new(RwLock::new(0));
        let cur_patmasks = Arc::new(RwLock::new(HashSet::new()));
        (
            PublisherImpl {
                next_id: 0,
                subscribers: subscribers.clone(),
                cur_tagmask: cur_tagmask.clone(),
                cur_patmasks: cur_patmasks.clone(),
            },
            subscribers.clone(),
            cur_tagmask.clone(),
            cur_patmasks.clone(),
        )
    }
    pub fn update_masks(&mut self) {
        let mut tagmask = 0;
        let mut patmasks = HashSet::new();
        for (_, handle) in self.subscribers.clone().lock().subscribers.iter() {
            tagmask |= handle.tagmask;
            for mask in &handle.patmasks {
                patmasks.insert(*mask);
            }
        }
        let mut t = self.cur_tagmask.write();
        *t = tagmask;
        let mut p = self.cur_patmasks.write();
        *p = patmasks;
    }
}

impl publisher::Server<::capnp::any_pointer::Owned> for PublisherImpl {
    fn subscribe(
        &mut self,
        params: publisher::SubscribeParams<::capnp::any_pointer::Owned>,
        mut results: publisher::SubscribeResults<::capnp::any_pointer::Owned>,
    ) -> Promise<(), ::capnp::Error> {
        println!("subscribe");

        // Gather subscription parameters
        let svc_rdr = pry!(pry!(params.get()).get_services());
        let tagmask = svc_rdr.reborrow().get_tagmask();
        let patmasks = match svc_rdr.reborrow().has_patmasks() {
            false => Vec::new(),
            true => pry!(svc_rdr.reborrow().get_patmasks()).iter().collect(),
        };
        let sub_client = pry!(pry!(params.get()).get_subscriber());

        // Insert new subscriber
        self.subscribers.lock().subscribers.insert(
            self.next_id,
            SubscriberHandle {
                client: sub_client,
                requests_in_flight: 0,
                tagmask,
                patmasks,
            },
        );

        // Update intersection of subscription patterns
        self.update_masks();

        results
            .get()
            .set_subscription(capnp_rpc::new_client(SubscriptionImpl::new(
                self.next_id,
                self.subscribers.clone(),
            )));

        self.next_id += 1;
        Promise::ok(())
    }
}

pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::net::ToSocketAddrs;
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {} server HOST:PORT", args[0]);
        return Ok(());
    }

    // spawn timer thread
    let (tx_event, rx_event) = flume::unbounded::<Event>();
    crate::timer::main(std::time::Duration::from_millis(500), tx_event)?;

    // controller data sharing
    let data = Arc::new(Mutex::new(PubData {
        duration: 0,
        tags: Box::new(capnp::message::Builder::new_default()),
        patcounts: HashMap::new(),
    }));


    let addr = args[2]
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");

    tokio::task::LocalSet::new()
        .run_until(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            let (
                publisher_impl,
                subscribers,
                cur_tagmask,
                cur_patmasks,
            ) = PublisherImpl::new();
            let publisher: publisher::Client<_> = capnp_rpc::new_client(publisher_impl);
            
            // spawn controller thread
            let (tx_pub, rx_pub) = flume::unbounded::<()>();
            let data1 = data.clone();
            std::thread::spawn( move || {
                crate::controller::main(
                    data1,
                    cur_tagmask.clone(),
                    cur_patmasks.clone(),
                    rx_event,
                    tx_pub,
                ).unwrap();
            });

            let handle_incoming = async move {
                loop {
                    let (stream, _) = listener.accept().await?;
                    stream.set_nodelay(true)?;
                    let (reader, writer) =
                        tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
                    let network = twoparty::VatNetwork::new(
                        reader,
                        writer,
                        rpc_twoparty_capnp::Side::Server,
                        Default::default(),
                    );
                    let rpc_system =
                        RpcSystem::new(Box::new(network), Some(publisher.clone().client));

                    tokio::task::spawn_local(Box::pin(rpc_system.map(|_| ())));
                }
            };

            let send_to_subscribers = async move {
                while let Ok(()) = rx_pub.recv_async().await {
                    let subscribers1 = subscribers.clone();
                    let subs = &mut subscribers.lock().subscribers;
                    for (&idx, mut subscriber) in subs.iter_mut() {
                        if subscriber.requests_in_flight < 5 {
                            subscriber.requests_in_flight += 1;
                            let mut request = subscriber.client.push_message_request();

                            let mut msg = capnp::message::Builder::new_default();
                            let mut msg_bdr = msg.init_root::<service_pub::Builder>();

                            let data1 = data.clone();
                            let data = data1.lock();

                            let mut tag_bdr = msg_bdr.reborrow().init_tags();
                            tag_bdr.reborrow().set_duration(data.duration);
                            tag_bdr.reborrow().set_tagmask(u16::MAX);
                            tag_bdr.reborrow().set_tags(data.tags.get_root_as_reader()?)?;

                            let mut pats_bdr = msg_bdr.init_pats(data.patcounts.len() as u32);
                            for (i, (&pat, &ct)) in data.patcounts.iter().enumerate() {
                                let mut pat_bdr = pats_bdr.reborrow().get(i as u32);
                                pat_bdr.reborrow().set_mask(pat);
                                pat_bdr.reborrow().set_duration(data.duration);
                                pat_bdr.reborrow().set_count(ct);
                            }

                            request.get().set_message(msg.get_root_as_reader()?)?;

                            let subscribers2 = subscribers1.clone();
                            tokio::task::spawn_local(Box::pin(request.send().promise.map(
                                move |r| match r {
                                    Ok(_) => {
                                        subscribers2.lock().subscribers.get_mut(&idx).map(
                                            |ref mut s| {
                                                s.requests_in_flight -= 1;
                                            },
                                        );
                                    }
                                    Err(e) => {
                                        println!("Got error: {:?}. Dropping subscriber.", e);
                                        subscribers2.lock().subscribers.remove(&idx);
                                    }
                                },
                            )));
                        }
                    }
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            };

            let _: ((), ()) =
                futures::future::try_join(handle_incoming, send_to_subscribers).await?;
            Ok(())
        })
        .await
}
