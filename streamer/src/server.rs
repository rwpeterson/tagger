// Pubsub pattern adapted from the capnproto-rust pubsub example code at
// https://github.com/capnproto/capnproto-rust/blob/master/capnp-rpc/examples/pubsub/server.rs
// Portions copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors

use capnp::message;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use either::Either;
use futures::{AsyncReadExt, FutureExt};
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tagger_capnp::tag_server_capnp::{publisher, service_pub};

#[allow(unused_imports)]
use tracing::{debug, error, info, span, warn, Instrument, Level};

use crate::data::WIN_DEFAULT;
use crate::processor;
use crate::rpc::PublisherImpl;
use crate::CliArgs;

// A large heap-allocated block to (re)use as the first segment in the
// capnp allocator; improves performance via reuse and minimization of
// zeroing out during initialization of new memory
const FIRST_SEGMENT_WORDS: usize = 1 << 24; // 2^24 words = 128 MiB

pub async fn main(args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    // broadcast channel for shutdown
    let (shutdown_sender, mut shutdown_receiver) = tokio::sync::broadcast::channel::<()>(1);

    // spawn timer thread
    let (sender_timer, receiver_timer) = flume::bounded(1);
    let (sender_event, receiver_event) = flume::unbounded();
    crate::timer::main(sender_timer.clone())?;

    let addr = args
        .addr
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
                global_window,
            ) = PublisherImpl::new(sender_event.clone(), args.clone());
            let publisher: publisher::Client<_> = capnp_rpc::new_client(publisher_impl);

            // spawn controller thread
            let (sender_raw, receiver_raw) = flume::bounded(5);
            let shutdown_sender_2 = shutdown_sender.clone();
            let (ct, cp, gw) =
            (cur_tagmask.clone(), cur_patmasks.clone(), global_window.clone());
            std::thread::spawn(move || {
                let cs =
                    crate::controller::main(
                        args,
                        receiver_timer,
                        receiver_event,
                        sender_raw,
                        ct,
                        cp,
                        gw,
                    );
                match cs {
                    Ok(()) => {}
                    Err(_) => {
                        let _ = shutdown_sender_2.send(());
                    }
                }
            });

            let (sender_proc, receiver_proc) = flume::unbounded();
            processor::main(
                receiver_raw,
                sender_proc,
                cur_tagmask.clone(),
                cur_patmasks.clone(),
            )?;

            let handle_incoming = async move {
                tokio::select! {
                    _ = async {
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
                        #[allow(unreachable_code)]
                        Ok::<_, std::io::Error>(())
                    } => {}
                    _ = shutdown_receiver.recv() => {}
                }
                Ok(())
            };

            let mut shutdown_receiver_2 = shutdown_sender.subscribe();
            let send_to_subscribers = async move {
                tokio::select! {
                    _ = async {
                        // Use one allocator, don't make a new one each loop
                        // Additionally, the user-supplied buffer for the first segment
                        // reduces cost of zeroing-out new memory allocations
                        let mut b = capnp::Word::allocate_zeroed_vec(FIRST_SEGMENT_WORDS);
                        let mut alloc = message::ScratchSpaceHeapAllocator::new(
                            capnp::Word::words_to_bytes_mut(&mut b),
                        );
                        let emptyvec = Arc::new(Vec::new());
                        while let Ok(pubdata) = receiver_proc.recv_async().await {
                            let (dur, tags, patcounts) = match pubdata {
                                Either::Left(t) => (t.dur, t.tags.clone(), t.counts),
                                Either::Right(l) => (l.dur, emptyvec.clone(), l.counts),
                            };

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
                                        tag_bdr.reborrow().set_channel(tag.channel.into());
                                    }

                                    let mut pats_bdr = msg_bdr.init_pats(patcounts.len() as u32);
                                    for (i, ((pat, win), &ct)) in patcounts.iter().enumerate() {
                                        let mut pat_bdr = pats_bdr.reborrow().get(i as u32);
                                        pat_bdr.reborrow().set_patmask(*pat);
                                        pat_bdr.reborrow().set_duration(dur);
                                        pat_bdr.reborrow().set_count(ct);
                                        pat_bdr.reborrow().set_window(win.unwrap_or(WIN_DEFAULT));
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
                                                info!("Dropping subscriber: {:?}", e);
                                                subscribers2.lock().subscribers.remove(&idx);
                                            }
                                        },
                                    )));
                                }
                            }
                        }
                        Ok::<(), Box<dyn std::error::Error>>(())
                    } => {}
                    _ = shutdown_receiver_2.recv() => {}
                }
                Ok::<_, std::io::Error>(())
            };

            let mut shutdown_receiver_3 = shutdown_sender.subscribe();
            let ctrl_c_watcher = async {
                tokio::select! {
                    ctrl_c_signal = tokio::signal::ctrl_c() => {match ctrl_c_signal {
                        Ok(()) => {
                            let span = span!(Level::INFO, "ctrl_c signal");
                            let _enter = span.enter();
                            info!("Manual shutdown signal received. Goodbye!");
                        }
                        Err(e) => {
                            let span = span!(Level::ERROR, "ctrl_c signal");
                            let _enter = span.enter();
                            error!("Unable to listen to shutdown signal: {}", e);
                        }
                    }}
                    _ = shutdown_receiver_3.recv() => {
                        let span = span!(Level::INFO, "automatic shutdown");
                        let _enter = span.enter();
                        info!("Automatic shutdown signal received. Goodbye!");
                    }
                }
                let _ = shutdown_sender.send(());
                Ok(())
            };

            let _: ((), (), ()) =
                futures::future::try_join3(handle_incoming, send_to_subscribers, ctrl_c_watcher)
                    .await?;
            Ok(())
        })
        .await
}
