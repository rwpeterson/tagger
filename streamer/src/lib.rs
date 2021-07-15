pub mod client;
pub mod controller;
pub mod data;
pub mod server;
pub mod timer;

pub enum Event {
    Tick,
    Set(InputSetting),
}

pub enum InputSetting {
    InversionMask(u16),
    Delay((u8, u32)),
    Threshold((u8, f64)),
}

pub mod bit {
    pub fn setbit16(num: &mut u16, bit: u8) {
        *num |= 1 << bit;
    }

    pub fn clearbit16(num: &mut u16, bit: u8) {
        *num &= !(1 << bit);
    }

    pub fn togglebit16(num: &mut u16, bit: u8) {
        *num ^= 1 << bit;
    }

    pub fn checkbit16(num: u16, bit: u8) -> bool {
        return (num >> bit) & 1 == 1;
    }

    pub fn changebit16(num: &mut u16, bit: u8, x: bool) {
        *num = (*num & !(1 << bit)) | ((x as u16) << bit);
    }

    pub fn setbit32(num: &mut u32, bit: u8) {
        *num |= 1 << bit;
    }

    pub fn clearbit32(num: &mut u32, bit: u8) {
        *num &= !(1 << bit);
    }

    pub fn togglebit32(num: &mut u32, bit: u8) {
        *num ^= 1 << bit;
    }

    pub fn checkbit32(num: &mut u32, bit: u8) -> bool {
        return (*num >> bit) & 1 == 1;
    }

    pub fn changebit32(num: &mut u32, bit: u8, x: bool) {
        *num = (*num & !(1 << bit)) | ((x as u32) << bit);
    }
}