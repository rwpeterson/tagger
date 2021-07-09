use capnp::message;
use std::collections::HashMap;
use tagtools::Tag;

#[allow(dead_code)]
pub struct TagPattern {
    tagmask: u16,
    duration: u64,
    tags: Vec<Tag>,
}

#[allow(dead_code)]
pub struct LogicPattern {
    patmask: u16,
    duration: u64,
    count: u64
}

/// Data from the tagger that needs to be passed between the controller and server.
/// Due to the size involved, we immediately create the capnp message for the tags
/// instead of passing them back and forth in memory several times first.
pub struct PubData {
    pub duration: u64,
    pub tags: Box<message::Builder<message::HeapAllocator>>,
    pub patcounts: HashMap<u16,u64>,
}