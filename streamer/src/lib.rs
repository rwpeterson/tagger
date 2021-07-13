pub mod tag_server_capnp;
pub mod tags_capnp;
pub mod controller;
pub mod timer;
pub mod data;
pub mod server;
pub mod client;

pub enum Event {
    Tick,
    Set(InputSettings),
}

pub struct InputSettings {
    channel: u8,
    negedge: Option<bool>,
    delay:   Option<u32>,
    voltage: Option<f64>,
}