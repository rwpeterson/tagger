use anyhow::Result;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use tagtools::CHAN16;
use timetag::ffi::new_logic_counter;

use crate::{Config, Event, GetImpl, Job, JobPayload, QueryImpl};

use crate::tag_server_capnp::JobStatus as CJobStatus;

#[derive(Clone)]
struct JobManager {
    waiting: Rc<RefCell<HashMap<u64, Job>>>,
    ready: Rc<RefCell<HashMap<u64, Job>>>,
    cancelled: Rc<RefCell<Vec<u64>>>,
    claimed: Rc<RefCell<Vec<u64>>>,
    cur_pats: Rc<RefCell<HashMap<u16, HashSet<u64>>>>, // patterns and their subscribing jobs
}

impl JobManager {
    pub fn new() -> Self {
        JobManager {
            waiting: Rc::new(RefCell::new(HashMap::new())),
            ready: Rc::new(RefCell::new(HashMap::new())),
            cancelled: Rc::new(RefCell::new(Vec::new())),
            claimed: Rc::new(RefCell::new(Vec::new())),
            cur_pats: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}


fn query_status(m: &JobManager, id: u64) -> CJobStatus {
    if m.waiting.borrow().contains_key(&id) {
        return CJobStatus::Waiting;
    } else if m.ready.borrow().contains_key(&id) {
        return CJobStatus::Ready;
    } else if m.cancelled.borrow().contains(&id) {
        return CJobStatus::Cancelled;
    } else if m.claimed.borrow().contains(&id) {
        return CJobStatus::Claimed;
    } else {
        return CJobStatus::Badid;
    }
}

pub fn logic(_cfg: Config, rx: flume::Receiver<Event>) -> Result<()> {
    let t = new_logic_counter();
    t.open();
    for ch in CHAN16 {
        t.set_input_threshold(ch, 2.0);
    }
    t.switch_logic_mode();
    let m = JobManager::new();
    loop {
        match rx.recv() {
            Ok(Event::Tick) => {
                t.read_logic();
                let dur = t.get_time_counter();
                for (pat, subs) in m.cur_pats.clone().borrow().iter() {
                    let counts = t.calc_count_pos(*pat);
                    for sub in subs {
                        if let Some(job) = m.waiting.clone().borrow_mut().get_mut(sub) {
                            if job.started == true {
                                if let Some(i) = job.patterns.iter().position(|x| x == pat) {
                                    job.events[i] += counts as u64;
                                }
                            }
                        }
                    }
                }
                let w_ptr = m.waiting.clone();
                let mut w = w_ptr.borrow_mut();
                let r_ptr = m.ready.clone();
                let mut r = r_ptr.borrow_mut();
                let mut readylist: Vec<u64> = Vec::new();
                for (id, job) in w.iter_mut() {
                    match job.started {
                        false => job.started = true, // start new jobs on full tick period
                        true => {
                            job.duration += dur;
                            job.cycles -= 1;
                            if job.cycles == 0 {
                                readylist.push(*id);
                            }
                        },
                    }
                }
                for rid in readylist {
                    if let Some(job) = w.remove(&rid) {
                        r.insert(rid, job);
                    }
                }
            }
            Ok(Event::Work(job)) => {
                let w_ptr = m.waiting.clone();
                let mut w = w_ptr.borrow_mut();
                if w.contains_key(&job.id) == false {
                    for pat in &job.patterns {
                        let hm_ptr = m.cur_pats.clone();
                        let mut hm = hm_ptr.borrow_mut();
                        match hm.get_mut(pat) {
                            Some(hs) => {
                                hs.insert(job.id);
                            }
                            None => {
                                let mut hs = HashSet::new();
                                hs.insert(job.id);
                                hm.insert(pat.to_owned(), hs);
                            }
                        }
                    }
                    w.insert(job.id, job);
                }
            }
            Ok(Event::Query(QueryImpl { id, tx })) => {
                tx.send(query_status(&m, id))?;
            }
            Ok(Event::Get(GetImpl { id, tx })) => {
                if let Some(job) = m.ready.clone().borrow_mut().remove(&id) {
                    m.claimed.clone().borrow_mut().push(id);
                    tx.send(JobPayload::Payload(job))?;
                } else {
                    tx.send(JobPayload::BadQuery(query_status(&m, id)))?;
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}
