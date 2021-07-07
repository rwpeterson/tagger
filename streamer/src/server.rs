// Adapted from the capnproto-rust pubsub example code at 
// https://github.com/capnproto/capnproto-rust/blob/master/capnp-rpc/examples/pubsub/server.rs
// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

use capnp_rpc::{RpcSystem, twoparty, rpc_twoparty_capnp, pry};
use crate::tag_server_capnp::{publisher, subscriber, subscription, service_pub};

use capnp::capability::Promise;

use futures::{AsyncReadExt, FutureExt, StreamExt};

struct SubscriberHandle {
    client: subscriber::Client<::capnp::any_pointer::Owned>,
    requests_in_flight: i32,
}

struct SubscriberMap {
    subscribers: HashMap<u64, SubscriberHandle>,
}

impl SubscriberMap {
    fn new() -> SubscriberMap {
        SubscriberMap { subscribers: HashMap::new() }
    }
}

struct SubscriptionImpl {
    id: u64,
    subscribers: Rc<RefCell<SubscriberMap>>,
}

impl SubscriptionImpl {
    fn new(id: u64, subscribers: Rc<RefCell<SubscriberMap>>) -> SubscriptionImpl {
        SubscriptionImpl { id: id, subscribers: subscribers }
    }
}

impl Drop for SubscriptionImpl {
    fn drop(&mut self) {
        println!("subscription dropped");
        self.subscribers.borrow_mut().subscribers.remove(&self.id);
    }
}

impl subscription::Server for SubscriptionImpl {}

struct PublisherImpl {
    next_id: u64,
    subscribers: Rc<RefCell<SubscriberMap>>,
}

impl PublisherImpl {
    pub fn new() -> (PublisherImpl, Rc<RefCell<SubscriberMap>>) {
        let subscribers = Rc::new(RefCell::new(SubscriberMap::new()));
        (PublisherImpl { next_id: 0, subscribers: subscribers.clone() },
         subscribers.clone())
    }
}

impl publisher::Server<::capnp::any_pointer::Owned> for PublisherImpl {
    fn subscribe(&mut self,
                 params: publisher::SubscribeParams<::capnp::any_pointer::Owned>,
                 mut results: publisher::SubscribeResults<::capnp::any_pointer::Owned>,)
                 -> Promise<(), ::capnp::Error>
    {
        println!("subscribe");
        self.subscribers.borrow_mut().subscribers.insert(
            self.next_id,
            SubscriberHandle {
                client: pry!(pry!(params.get()).get_subscriber()),
                requests_in_flight: 0,
            }
        );

        results.get().set_subscription(capnp_rpc::new_client(
            SubscriptionImpl::new(self.next_id, self.subscribers.clone())));

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

    let addr = args[2].to_socket_addrs().unwrap().next().expect("could not parse address");

    tokio::task::LocalSet::new().run_until(async move {
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let (publisher_impl, subscribers) = PublisherImpl::new();
        let publisher: publisher::Client<_> = capnp_rpc::new_client(publisher_impl);

        let handle_incoming = async move {
            loop {
                let (stream, _) = listener.accept().await?;
                stream.set_nodelay(true)?;
                let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
                let network =
                    twoparty::VatNetwork::new(reader, writer,
                                              rpc_twoparty_capnp::Side::Server, Default::default());
                let rpc_system = RpcSystem::new(Box::new(network), Some(publisher.clone().client));

                tokio::task::spawn_local(Box::pin(rpc_system.map(|_| ())));
            }
        };

        // Trigger sending approximately once per second.
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<()>();
        std::thread::spawn(move || {
            while let Ok(()) = tx.unbounded_send(()) {
                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        });

        let send_to_subscribers = async move {
            while let Some(()) = rx.next().await {
                let subscribers1 = subscribers.clone();
                let subs = &mut subscribers.borrow_mut().subscribers;
                for (&idx, mut subscriber) in subs.iter_mut() {
                    if subscriber.requests_in_flight < 5 {
                        subscriber.requests_in_flight += 1;
                        let mut request = subscriber.client.push_message_request();

                        let mut msg = capnp::message::Builder::new_default();
                        let msg_bdr = msg.init_root::<service_pub::Builder>();
                        let mut pat_bdr = msg_bdr.init_pats(1);
                        pat_bdr.reborrow().get(0).set_mask(1);
                        pat_bdr.reborrow().get(0).set_count(69);
                        pat_bdr.reborrow().get(0).set_duration(420);
                        request.get().set_message(msg.get_root_as_reader()?)?;
                        let subscribers2 = subscribers1.clone();
                        tokio::task::spawn_local(Box::pin(
                            request.send().promise.map(move |r| {
                                match r {
                                    Ok(_) => {
                                        subscribers2.borrow_mut().subscribers.get_mut(&idx).map(|ref mut s| {
                                            s.requests_in_flight -= 1;
                                        });
                                    }
                                    Err(e) => {
                                        println!("Got error: {:?}. Dropping subscriber.", e);
                                        subscribers2.borrow_mut().subscribers.remove(&idx);
                                    }
                                }
                            })));
                    }
                }
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        };

        let _ : ((), ()) = futures::future::try_join(handle_incoming, send_to_subscribers).await?;
        Ok(())
    }).await
}
