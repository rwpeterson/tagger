use anyhow::Result;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use tagtools::CHAN16;
use timetag::ffi::{new_logic_counter, LogicCounter};
use cxx::SharedPtr;

use crate::{Config, Event, GetImpl, Job, JobPayload, QueryImpl};

use crate::tag_server_capnp::JobStatus as CJobStatus;

#[derive(Clone)]
struct JobManager {
    waiting: HashMap<u64, Job>,
    ready: HashMap<u64, Job>,
    cancelled: Vec<u64>,
    claimed: Vec<u64>,
    cur_pats: HashMap<u16, HashSet<u64>>, // patterns and their subscribing jobs
}

impl JobManager {
    pub fn new() -> Self {
        JobManager {
            waiting: HashMap::new(),
            ready: HashMap::new(),
            cancelled: Vec::new(),
            claimed: Vec::new(),
            cur_pats: HashMap::new(),
        }
    }
}

pub fn logic(_cfg: Config, rx: flume::Receiver<Event>) -> Result<()> {
    let t = new_logic_counter();
    t.open();
    for ch in CHAN16 {
        t.set_input_threshold(ch, 2.0);
    }
    t.set_fg(200_000, 100_000);
    t.switch_logic_mode();
    let m = Rc::new(RefCell::new(JobManager::new()));
    loop { match rx.recv() {
        Ok(Event::Tick) => {
            t.clone().read_logic();
            update_subs(Rc::clone(&m), t.clone());
            add_duration(Rc::clone(&m), t.clone());
            reap_ready_jobs(Rc::clone(&m));
            activate_new_jobs(Rc::clone(&m));
            update_patterns(Rc::clone(&m));
        }
        Ok(Event::Work(job)) => {
            add_job(Rc::clone(&m), job);
            update_patterns(Rc::clone(&m));
        }
        Ok(Event::Query(QueryImpl { id, tx })) => {
            tx.send(query_status(Rc::clone(&m), id))?;
        }
        Ok(Event::Get(GetImpl { id, tx })) => {
            ship_job(Rc::clone(&m), id, tx)?;
        }
        Err(_) => break,
    }}
    Ok(())
}


fn query_status(m: Rc<RefCell<JobManager>>, id: u64) -> CJobStatus {
    let mgr = m.borrow();
    if mgr.waiting.contains_key(&id) {
        return CJobStatus::Waiting;
    } else if mgr.ready.contains_key(&id) {
        return CJobStatus::Ready;
    } else if mgr.cancelled.contains(&id) {
        return CJobStatus::Cancelled;
    } else if mgr.claimed.contains(&id) {
        return CJobStatus::Claimed;
    } else {
        return CJobStatus::Badid;
    }
}

fn update_subs(m: Rc<RefCell<JobManager>>, t: SharedPtr<LogicCounter>) {
    let mut mgr = RefCell::borrow_mut(&m);
    for (pat, subs) in mgr.cur_pats.clone() {
        let cts = t.calc_count_pos(pat);
        for sub in subs {
            if let Some(job) = mgr.waiting.get_mut(&sub) {
                if job.started {
                    if let Some(i) = job.patterns.iter().position(|&x| x == pat) {
                        match job.events.get_mut(i) {
                            Some(ct) => *ct += cts as u64,
                            None => eprintln!("Tried to access missing event {}", i),
                        }
                    }
                }
            }
        }
    }
}

fn add_duration(m: Rc<RefCell<JobManager>>, t: SharedPtr<LogicCounter>) {
    let dur = t.get_time_counter();
    let mut mgr = RefCell::borrow_mut(&m);
    for (_, job) in mgr.waiting.iter_mut() {
        if job.started {
            job.duration += dur;
            job.cycles -= 1;
        }
    }
}

fn reap_ready_jobs(m: Rc<RefCell<JobManager>>) {
    let mut mgr = RefCell::borrow_mut(&m);
    for (id, job) in mgr.waiting.clone() {
        if job.cycles == 0 {
            let rj = mgr.waiting.remove(&id).unwrap();
            mgr.ready.insert(id, rj);
        }
    }
}

fn activate_new_jobs(m: Rc<RefCell<JobManager>>) {
    let mut mgr = RefCell::borrow_mut(&m);
    for (_, job) in mgr.waiting.iter_mut() {
        if !job.started {
            job.started = true;
        }
    }    
}

fn add_job(m: Rc<RefCell<JobManager>>, job: Job) {
    let mut mgr = RefCell::borrow_mut(&m);
    if mgr.waiting.contains_key(&job.id) == false {
        mgr.waiting.insert(job.id, job);
    }
}

fn update_patterns(m: Rc<RefCell<JobManager>>) {
    let mut mgr = RefCell::borrow_mut(&m);
    for (id, job) in mgr.waiting.clone() {
        for pat in job.patterns {
            match mgr.cur_pats.get_mut(&pat) {
                Some(hs) => {
                    hs.insert(id);
                }
                None => {
                    let mut hs = HashSet::new();
                    hs.insert(id);
                    mgr.cur_pats.insert(pat, hs);
                }
            }
        }
    }
}

fn ship_job(m: Rc<RefCell<JobManager>>, id: u64, tx: flume::Sender<JobPayload>) -> Result<()> {
    let mut mgr = RefCell::borrow_mut(&m);
    if let Some(job) = mgr.ready.remove(&id) {
        mgr.claimed.push(id);
        tx.send(JobPayload::Payload(job))?;
    } else {
        tx.send(JobPayload::BadQuery(query_status(Rc::clone(&m), id)))?;
    }
    Ok(())
}