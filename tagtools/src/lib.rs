pub mod de;
pub mod pat;
pub mod ser;
pub mod tags_capnp;

/// The basic representation of a tagged event
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct Tag {
    /// Counter in time units from arbitrary offset
    pub time: i64,
    /// Channel (1-indexed) of the event
    pub channel: u8,
}

/// Representation for two-dimensional data like histograms, etc.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Bin<T, U>
where 
    T: std::str::FromStr,
    U: std::str::FromStr,
{
    pub x: T,
    pub y: U,
}

pub const TSTEP: f64 = 156.25e-12;
pub const CHAN16: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

