use capnp::capability::Promise;
use capnp_rpc::pry;
use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tagger_capnp::tag_server_capnp::{
    input_settings, publisher, service_sub, subscriber, subscription, Mode,
};
use tagtools::bit::BitOps;

#[allow(unused_imports)]
use tracing::{debug, error, info, span, warn, Instrument, Level};

use crate::data::WIN_DEFAULT;
use crate::{CliArgs, Event, InputSetting};

pub struct SubscriberHandle {
    pub client: subscriber::Client<::capnp::any_pointer::Owned>,
    pub requests_in_flight: i32,
    pub tagmask: u16,
    pub patmasks: Vec<(u16, Option<u32>)>,
}

pub struct SubscriberMap {
    pub subscribers: HashMap<u64, SubscriberHandle>,
}

impl SubscriberMap {
    pub fn new() -> SubscriberMap {
        SubscriberMap {
            subscribers: HashMap::new(),
        }
    }
}

pub struct SubscriptionImpl {
    pub id: u64,
    pub subscribers: Arc<Mutex<SubscriberMap>>,
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
        let span = span!(Level::INFO, "subscription_drop");
        let _enter = span.enter();
        self.subscribers.lock().subscribers.remove(&self.id);
        info!("subscription dropped");
    }
}

impl subscription::Server for SubscriptionImpl {}

pub struct PublisherImpl {
    // Subscription state
    next_id: u64,
    subscribers: Arc<Mutex<SubscriberMap>>,

    // Union of subscriber's data subscriptions
    cur_tagmask: Arc<RwLock<u16>>,
    cur_patmasks: Arc<RwLock<HashSet<(u16, Option<u32>)>>>,

    // State management of input properties
    // (tagger API has individual setters and global getter; vendor provides only setters)
    invmask: Arc<RwLock<u16>>,
    delays: Arc<RwLock<Vec<u32>>>,
    thresholds: Arc<RwLock<Vec<f64>>>,

    // State management of global window for logic mode
    global_window: Arc<RwLock<Option<u32>>>,

    // Send Event::Set commands to controller
    tx_controller: flume::Sender<Event>,

    // CLI args which may override certain API options
    args: CliArgs,
}

impl PublisherImpl {
    pub fn new(
        tx_controller: flume::Sender<Event>,
        args: CliArgs,
    ) -> (
        PublisherImpl,
        Arc<Mutex<SubscriberMap>>,
        Arc<RwLock<u16>>,
        Arc<RwLock<HashSet<(u16, Option<u32>)>>>,
        Arc<RwLock<Option<u32>>>,
    ) {
        let subscribers = Arc::new(Mutex::new(SubscriberMap::new()));
        let cur_tagmask = Arc::new(RwLock::new(0));
        let cur_patmasks = Arc::new(RwLock::new(HashSet::new()));
        let global_window = match args.logic {
            // In logic mode, there must be a global window state
            true => match args.window {
                None => Arc::new(RwLock::new(Some(WIN_DEFAULT))),
                Some(x) => Arc::new(RwLock::new(Some(x))),
            }
            // In tag mode, there is no need for a global window
            false => Arc::new(RwLock::new(None)),
        };
        (
            PublisherImpl {
                next_id: 0,
                subscribers: subscribers.clone(),
                cur_tagmask: cur_tagmask.clone(),
                cur_patmasks: cur_patmasks.clone(),
                invmask: Arc::new(RwLock::new(0)),
                delays: Arc::new(RwLock::new(vec![0; 16])),
                thresholds: Arc::new(RwLock::new(vec![2.0; 16])),
                global_window: global_window.clone(), 
                tx_controller,
                args,
            },
            subscribers.clone(),
            cur_tagmask.clone(),
            cur_patmasks.clone(),
            global_window.clone(),
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

        let span = span!(Level::INFO, "subscribe");
        let _enter = span.enter();

        // Gather subscription parameters
        let svc_rdr = pry!(pry!(params.get()).get_services());
        let tagmask = svc_rdr.reborrow().get_tagmask();
        let prdr = svc_rdr.reborrow().get_patmasks();
        let patmasks: Vec<(u16, Option<u32>)> = match pry!(prdr.which()) {
            p::Bare(b) => {
                let rdr = pry!(b);
                let p = rdr.iter().map(|p| (p, None)).collect();
                p
            }
            p::Windowed(w) => {
                let rdr = pry!(w);
                let p = rdr
                    .iter()
                    .map(|lrdr| {
                        let pm = lrdr.reborrow().get_patmask();
                        let wd = lrdr.reborrow().get_window();
                        match wd {
                            // The subscriber doesn't specify, they get what they get
                            0 => (pm, None),
                            w => match self.args.window {
                                None => {
                                    match self.args.logic {
                                        true => {
                                            // Accept window as new global window
                                            let mut gw = self.global_window.write();
                                            if w == 0 {
                                                *gw = Some(WIN_DEFAULT)
                                            } else {
                                                *gw = Some(w)
                                            }
                                            (pm, None)
                                        },
                                        false => {
                                            // Any individual pattern can have whatever window it wants
                                            (pm, Some(w))
                                        }
                                    }
                                }
                                Some(_) => {
                                    // Ignore requested window for subscription,
                                    // e.g. when in logic mode and there can only be one
                                    // Must use dedicated get/set for global window
                                    (pm, None)
                                }
                            },
                        }
                    })
                    .collect();
                p
            }
        };

        let sub_client = pry!(pry!(params.get()).get_subscriber());

        info!("mask {:x?}", patmasks.clone());

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
        let span = span!(Level::INFO, "set_input");
        let _enter = span.enter();
        match pry!(pry!(pry!(params.get()).get_s()).which()) {
            w::Inversion(r) => {
                let rdr = pry!(r);
                let ch = rdr.get_ch();
                let inv = rdr.get_inv();
                info!("channel {}, inversion {}", ch, inv,);
                let mut invmask = self.invmask.write();
                invmask.change(ch as usize, inv);
                self.tx_controller
                    .send(Event::Set(InputSetting::InversionMask(*invmask)))
                    .unwrap();
            }
            w::Delay(r) => {
                let rdr = pry!(r);
                let mut delays = self.delays.write();
                let ch = rdr.get_ch();
                let del = rdr.get_del();
                info!("channel {}, delay {}", ch, del,);
                delays[(ch - 1) as usize] = del;
                self.tx_controller
                    .send(Event::Set(InputSetting::Delay((ch, del))))
                    .unwrap();
            }
            w::Threshold(r) => {
                let rdr = pry!(r);
                let mut thresholds = self.thresholds.write();
                let ch = rdr.get_ch();
                let th = rdr.get_th();
                info!("channel {}, threshold {} V", ch, th,);
                thresholds[(ch - 1) as usize] = th;
                self.tx_controller
                    .send(Event::Set(InputSetting::Threshold((ch, th))))
                    .unwrap();
            }
        }
        Promise::ok(())
    }

    fn get_inputs(
        &mut self,
        _params: publisher::GetInputsParams<::capnp::any_pointer::Owned>,
        mut results: publisher::GetInputsResults<::capnp::any_pointer::Owned>,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let span = span!(Level::INFO, "get_inputs");
        let _enter = span.enter();

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

        info!("processed");

        Promise::ok(())
    }

    fn query_mode(
        &mut self,
        _params: publisher::QueryModeParams<::capnp::any_pointer::Owned>,
        mut results: publisher::QueryModeResults<::capnp::any_pointer::Owned>,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let span = span!(Level::INFO, "query_mode");
        let _enter = span.enter();
        match self.args.logic {
            // tag mode
            false => {
                info!("Operating in timetag mode");
                results.get().set_m(Mode::Timetag)
            },
            // logic mode
            true => {
                info!("Operating in logic mode");
                results.get().set_m(Mode::Logic)
            },
        }
        Promise::ok(())
    }

    fn set_window(
        &mut self,
        params: publisher::SetWindowParams<::capnp::any_pointer::Owned>,
        _results: publisher::SetWindowResults<::capnp::any_pointer::Owned>,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let span = span!(Level::INFO, "set_window");
        let _enter = span.enter();
        let rdr = pry!(params.get());
        let w = rdr.get_w();
        let mut gw = self.global_window.write();
        *gw = Some(w);
        self.tx_controller.send(Event::Set(InputSetting::Window(w))).unwrap();
        info!("Set global window to {}", w);
        Promise::ok(())
    }

    fn get_window(
        &mut self,
        _params: publisher::GetWindowParams<::capnp::any_pointer::Owned>,
        mut results: publisher::GetWindowResults<::capnp::any_pointer::Owned>,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let span = span!(Level::INFO, "get_window");
        let _enter = span.enter();
        let gw = self.global_window.read();
        match *gw {
            Some(w) => {
                info!("Global window is {}", w);
                results.get().set_w(w);
            },
            None => {
                info!("There is no global window active");
                results.get().set_w(0);
            },
        }
        Promise::ok(())
    }
}