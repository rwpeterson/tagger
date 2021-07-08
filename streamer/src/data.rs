use std::collections::HashMap;
use tagtools::Tag;

pub struct TagPattern {
    tagmask: u16,
    duration: u64,
    tags: Vec<Tag>,
}

pub struct LogicPattern {
    patmask: u16,
    duration: u64,
    count: u64
}

pub struct PubData {
    pub duration: u64,
    pub tags: Vec<Tag>,
    pub patcounts: HashMap<u16,u64>,
}