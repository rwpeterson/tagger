pub mod server;
pub mod controller;
pub mod tag_server_capnp;
pub mod timer;

use capnp::capability::Promise;
use capnp_rpc::pry;
use flume::Sender;
use std::sync::atomic::{AtomicU64, Ordering};

use tag_server_capnp::tagger;
use tag_server_capnp::JobStatus as CJobStatus;
use tag_server_capnp::Resolution as CResolution;

#[derive(Clone, Copy)]
pub struct Config {
    pub addr: std::net::SocketAddr,
    pub rate: u16,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            addr: "127.0.0.1:6969".parse().unwrap(),
            rate: 400,
        }
    }
}

impl Config {
    /// Convert server rate to tagger's tick units of 5 ns
    fn get_rate_tick_units(self) -> u64 {
        return 200_000 * self.rate as u64;
    }
}

pub enum Event {
    Tick,
    Work(Job),
    Query(QueryImpl),
    Get(GetImpl),
}

pub struct QueryImpl {
    pub id: u64,
    pub tx: flume::Sender<CJobStatus>,
}

pub struct GetImpl {
    pub id: u64,
    pub tx: flume::Sender<JobPayload>,
}

pub enum JobPayload {
    BadQuery(CJobStatus),
    Payload(Job),
}

#[derive(Clone)]
pub struct Job {
    pub id: u64,
    pub patterns: Vec<u16>,
    pub events: Vec<u64>,
    pub window: i64,
    pub duration: u64, // Requested and actual duration in tagger tick units of 5 ns
    pub cycles: u64,   // Number of cycles to acquire data
    pub started: bool,
    pub finished: bool,
    pub starttag: i64,
    pub stoptag: i64,
    pub meta: JobMeta,
    pub resolution: CResolution,
    pub handle: String,
}

impl Default for Job {
    fn default() -> Self {
        Job {
            id: 0,
            patterns: Vec::new(),
            events: Vec::new(),
            window: 1,
            duration: 0,
            cycles: 0,
            started: false,
            finished: false,
            starttag: 0,
            stoptag: 0,
            meta: JobMeta::Purgatory,
            resolution: CResolution::Norm,
            handle: String::new(),
        }
    }
}

#[derive(Clone)]
pub enum JobMeta {
    Submission,
    Ok,
    Err(String),
    Purgatory,  // internal impl state
    Ready,      // internal impl state
    InProgress, // internal impl state
}

pub struct TagServerImpl {
    pub cfg: Config,       // Global config
    pub id: AtomicU64,     // Next job gets this id
    pub tx: Sender<Event>,
}

impl TagServerImpl {
    pub fn nextid(&mut self) -> u64 {
        return self.id.fetch_add(1, Ordering::AcqRel);
    }
}

impl tagger::Server for TagServerImpl {
    fn savetags(
        &mut self,
        _: tagger::SavetagsParams<>,
        _: tagger::SavetagsResults<>,
    ) -> Promise<(), capnp::Error> {
        Promise::ok(())
    }

    fn submitjob(
        &mut self,
        params: tagger::SubmitjobParams<>,
        mut results: tagger::SubmitjobResults<>
    ) -> Promise<(), capnp::Error> {
        let job_rdr = pry!(pry!(params.get()).get_job());
        let dur = job_rdr.get_duration();
        let id = self.nextid();
        let period = self.cfg.get_rate_tick_units();
        let limit: u64 = 600_000 * 200_000_000; // 1 week
        let s = self.tx.send(
            Event::Work(Job {
                id,
                patterns: pry!(job_rdr.get_patterns()).iter().collect(),
                events: vec![0; pry!(job_rdr.get_patterns()).len() as usize],
                // window: default for now
                cycles: if dur < period {
                    1
                } else if dur > limit {
                    limit / period
                } else {
                    dur / period
                },
                finished: false,
                meta: JobMeta::Ready,
                handle: pry!(job_rdr.get_handle()).to_owned(),
                ..Default::default()
            })
        );
        match s {
            Ok(()) => {
                pry!(results.get().get_sub()).set_jobid(id);
            },
            Err(_) => {
                pry!(results.get().get_sub()).set_badsub(tag_server_capnp::JobStatus::Refused)
            },
        }
        Promise::ok(())
    }

    fn queryjobdone(
        &mut self,
        params: tagger::QueryjobdoneParams<>,
        mut results: tagger::QueryjobdoneResults<>
    ) -> Promise<(), capnp::Error> {
        let id = pry!(params.get()).get_jobid();
        let (tx, rx) = flume::unbounded();
        self.tx.send(Event::Query(QueryImpl { id, tx })).unwrap();
        let m = rx.recv().unwrap();
        results.get().set_ret(m);
        Promise::ok(())
    }

    fn getresults(
        &mut self,
        params: tagger::GetresultsParams<>,
        mut results: tagger::GetresultsResults<>
    ) -> Promise<(), capnp::Error> {
        let id = pry!(params.get()).get_jobid();
        let (tx, rx) = flume::unbounded();
        self.tx.send(Event::Get(GetImpl { id, tx })).unwrap();
        let m = rx.recv().unwrap();
        let bdr = results.get();
        match m {
            JobPayload::BadQuery(s) => {
                bdr.init_payload().set_badquery(s);
            },
            JobPayload::Payload(j) => {
                let mut jbdr = bdr.init_payload().init_payload();
                jbdr.reborrow().set_id(j.id);
                let patb = jbdr.reborrow().init_patterns(j.patterns.len() as u32);
                for (i, &value) in j.patterns.iter().enumerate() {
                    patb.reborrow().set(i as u32, value);
                }
                let evtb = jbdr.reborrow().init_events(j.events.len() as u32);
                for (i, &value) in j.events.iter().enumerate() {
                    evtb.reborrow().set(i as u32, value);
                }
                jbdr.reborrow().set_window(j.window);
                jbdr.reborrow().set_duration(j.duration);
                jbdr.reborrow().set_finished(j.finished);
                jbdr.reborrow().set_starttag(j.starttag);
                jbdr.reborrow().set_stoptag(j.stoptag);
                jbdr.reborrow().init_meta().set_ok(());
                jbdr.reborrow().set_resolution(j.resolution);
                jbdr.reborrow().set_handle(j.handle.as_str());
            },
        }
        Promise::ok(())
    }
}