// Adapted from the capnproto-rust pubsub example code at
// https://github.com/capnproto/capnproto-rust/blob/master/capnp-rpc/examples/pubsub/server.rs
// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors

use capnp::capability::Promise;
use capnp::message;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{AsyncReadExt, FutureExt};
use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, HashSet};
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tagger_capnp::tag_server_capnp::{
    input_settings, publisher, service_pub, service_sub, subscriber, subscription,
};
use tagtools::bit;

use crate::{Event, InputSetting, CliArgs};
use crate::processor;

const FIRST_SEGMENT_WORDS: usize = 1 << 24; // 2^24 words = 128 MiB

pub struct SubscriberHandle {
    client: subscriber::Client<::capnp::any_pointer::Owned>,
    requests_in_flight: i32,
    tagmask: u16,
    patmasks: Vec<(u16, Option<u32>)>,
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
    // Subscription state
    next_id: u64,
    subscribers: Arc<Mutex<SubscriberMap>>,

    // Union of subscriber's data subscriptions
    cur_tagmask: Arc<RwLock<u16>>,
    cur_patmasks: Arc<RwLock<HashSet<(u16, Option<u32>)>>>,

    // State management of input properties
    // (tagger API has individual setters and global getter)
    invmask: Arc<RwLock<u16>>,
    delays: Arc<RwLock<Vec<u32>>>,
    thresholds: Arc<RwLock<Vec<f64>>>,

    // Send Event::Set commands to controller
    tx_controller: flume::Sender<Event>,
}

impl PublisherImpl {
    pub fn new(
        tx_controller: flume::Sender<Event>,
    ) -> (
        PublisherImpl,
        Arc<Mutex<SubscriberMap>>,
        Arc<RwLock<u16>>,
        Arc<RwLock<HashSet<(u16, Option<u32>)>>>,
    ) {
        let subscribers = Arc::new(Mutex::new(SubscriberMap::new()));
        let cur_tagmask = Arc::new(RwLock::new(0));
        let cur_patmasks = Arc::new(RwLock::new(HashSet::new()));
        (
            PublisherImpl {
                next_id: 0,
                subscribers:  subscribers.clone(),
                cur_tagmask:  cur_tagmask.clone(),
                cur_patmasks: cur_patmasks.clone(),
                invmask:    Arc::new(RwLock::new(0)),
                delays:     Arc::new(RwLock::new(vec![0; 16])),
                thresholds: Arc::new(RwLock::new(vec![2.0; 16])),
                tx_controller,
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
        use service_sub::patmasks as p;

        // Gather subscription parameters
        let svc_rdr = pry!(pry!(params.get()).get_services());
        let tagmask = svc_rdr.reborrow().get_tagmask();
        let prdr = svc_rdr.reborrow().get_patmasks();
        let patmasks: Vec<(u16, Option<u32>)> = match pry!(prdr.which()) {
            p::Bare(b) => {
                let rdr = pry!(b);
                let p = rdr.iter().map(|p| (p, None)).collect();
                p
            },
            p::Windowed(w) => {
                let rdr = pry!(w);
                let p = rdr.iter().map(|lrdr| {
                    let pm = lrdr.reborrow().get_patmask();
                    let wd = lrdr.reborrow().get_window();
                    match wd {
                        0 => (pm, None),
                        w => (pm, Some(w)),
                    }
                }).collect();
                p
            },
        };

        print!("subscribe: [");
        for (pat, win) in patmasks.clone() {
            match win {
                Some(w) => print!("{:#x} ({}), ", pat, w),
                None => print!("{:#x}, ", pat),
            }
        }
        println!("]");

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

    fn set_input(
        &mut self,
        params: publisher::SetInputParams<::capnp::any_pointer::Owned>,
        _results: publisher::SetInputResults<::capnp::any_pointer::Owned>,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        use input_settings::Which as w;
        match pry!(pry!(pry!(params.get()).get_s()).which()) {
            w::Inversion(r) => {
                let rdr = pry!(r);
                let ch = rdr.get_ch();
                let inv = rdr.get_inv();
                println!(
                    "set channel {}: inversion mask {}",
                    ch,
                    inv,
                );
                let mut invmask = self.invmask.write();
                bit::changebit16(&mut *invmask, ch, inv);
                self.tx_controller.send(
                    Event::Set(InputSetting::InversionMask(*invmask))
                ).unwrap();
            },
            w::Delay(r) => {
                let rdr = pry!(r);
                let mut delays = self.delays.write();
                let ch = rdr.get_ch();
                let del = rdr.get_del();
                println!(
                    "set channel {}: delay {}",
                    ch,
                    del,
                );
                delays[(ch - 1) as usize] = del;
                self.tx_controller.send(
                    Event::Set(InputSetting::Delay((ch, del)))
                ).unwrap();
            },
            w::Threshold(r) => {
                let rdr = pry!(r);
                let mut thresholds = self.thresholds.write();
                let ch = rdr.get_ch();
                let th = rdr.get_th();
                println!(
                    "set channel {} threshold: {} V",
                    ch,
                    th,
                );
                thresholds[(ch - 1) as usize] = th;
                self.tx_controller.send(
                    Event::Set(InputSetting::Threshold((ch, th)))
                ).unwrap();
            },
        }
        Promise::ok(())
    }

    fn get_inputs(
        &mut self,
        _params: publisher::GetInputsParams<::capnp::any_pointer::Owned>,
        mut results: publisher::GetInputsResults<::capnp::any_pointer::Owned>
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let invmask = self.invmask.read();
        let delays = self.delays.read();
        let thresholds = self.thresholds.read();

        let mut bdr = results.get().init_s();
        bdr.set_inversionmask(*invmask);
        let mut d_bdr = bdr.reborrow().init_delays(delays.len() as u32);
        for (i, &d) in delays.iter().enumerate() {
            d_bdr.set(i as u32, d);
        }
        let mut t_bdr = bdr.reborrow().init_thresholds(thresholds.len() as u32);
        for (i, &t) in thresholds.iter().enumerate() {
            t_bdr.set(i as u32, t);
        }
        
        println!("get inputs request: ok");

        Promise::ok(())
    }
}

pub async fn main(args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    // spawn timer thread
    let (sender_timer, receiver_timer) = flume::bounded(1);
    let (sender_event, receiver_event) = flume::unbounded();
    crate::timer::main(sender_timer.clone())?;

    let addr = args.addr
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
            ) = PublisherImpl::new(sender_event.clone());
            let publisher: publisher::Client<_> = capnp_rpc::new_client(publisher_impl);

            // spawn controller thread
            let (sender_raw, receiver_raw) = flume::bounded(5);
            //let (sender_tag, receiver_tag) = flume::unbounded();
            let (sender_proc, receiver_proc) = flume::unbounded();
            std::thread::spawn(move || {
                crate::controller::main(
                    receiver_timer,
                    receiver_event,
                    sender_raw,
                )
                .unwrap();
            });
            
            //copier::main(receiver_raw, sender_tag)?;
            processor::main(receiver_raw,
                sender_proc,
                cur_tagmask.clone(),
                cur_patmasks.clone(),
            )?;

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
                // Use one allocator, don't make a new one each loop
                // Additionally, the user-supplied buffer for the first segment
                // reduces cost of zeroing-out new memory allocations
                let mut b = capnp::Word::allocate_zeroed_vec(FIRST_SEGMENT_WORDS);
                let mut alloc = message::ScratchSpaceHeapAllocator::new(
                    capnp::Word::words_to_bytes_mut(&mut b)
                );
                while let Ok((dur, tags, patcounts)) = receiver_proc.recv_async().await {
                    let subscribers1 = subscribers.clone();
                    let subs = &mut subscribers.lock().subscribers;
                    
                    for (&idx, mut subscriber) in subs.iter_mut() {
                        if subscriber.requests_in_flight < 5 {
                            subscriber.requests_in_flight += 1;

                            // Only make the message if the sub isn't swamped
                            let mut msg = capnp::message::Builder::new(&mut alloc);
                            let mut msg_bdr = msg.init_root::<service_pub::Builder>();

                            let mut tag_bdr = msg_bdr.reborrow().init_tags();
                            tag_bdr.reborrow().set_duration(dur);
                            tag_bdr.reborrow().set_tagmask(subscriber.tagmask);
                            let outer_bdr = tag_bdr.reborrow().init_tags().init_tags(1);
                            let mut inner_bdr = outer_bdr.init(0, tags.len() as u32);
                            for (i, tag) in tags.iter().enumerate() {
                                let mut tag_bdr = inner_bdr.reborrow().get(i as u32);
                                tag_bdr.reborrow().set_time(tag.time);
                                tag_bdr.reborrow().set_channel(tag.channel);
                            }

                            let mut pats_bdr = msg_bdr.init_pats(patcounts.len() as u32);
                            for (i, ((pat, win), &ct)) in patcounts.iter().enumerate() {
                                let mut pat_bdr = pats_bdr.reborrow().get(i as u32);
                                pat_bdr.reborrow().set_patmask(*pat);
                                pat_bdr.reborrow().set_duration(dur);
                                pat_bdr.reborrow().set_count(ct);
                                pat_bdr.reborrow().set_window(win.unwrap_or_default());
                            }

                            let mut request = subscriber.client.push_message_request();

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
