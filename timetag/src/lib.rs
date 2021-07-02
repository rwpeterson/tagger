//! A Rust wrapper for dotfast consulting's `CTimeTag` library.
//!
//! `timetag` supports both tag mode and logic mode.  Bridging between
//! C++ and Rust is done using [CXX](https://cxx.rs), which allows for
//! high-level, safe interoperability.  However, CXX only supports a
//! subset of C++ written in a modern style with smart pointers.
//! Because the vendor library is written using a C-style raw pointer
//! interface, the `taghelper.h` library is provided, which wraps the
//! the vendor classes with an additional level of indirection so that
//! CXX's constraints can be satisfied.  This should have zero or
//! negligible overhead in general, but the indirection does mean that
//! an additional copy of tag data must be made:
//!
//! ```text
//! Owned by CTimeTag:          Owned by TimeTagger:          Application logic:
//! +----------------+          +------------------+          +------------------+
//! | TimeType*      |   Copy   | std::unique_ptr< |   Copy   | [Save to disk]   |
//! | ChannelType*   | =======> |   std::vector<   | =======> | [Display]        |
//! |                |    ^     |     FFiTag>>     |    ^     | [Etc.]           |
//! +----------------+    |     +------------------+    |     +------------------+
//!                       |                             |
//!         This copy is necessary due to          This copy is always necessary
//!         indirection: Rust cannot access the    for application logic, even in
//!         raw pointers directly, so data must    a pure C++ program, as data in
//!         be copied into the wrapper class       the raw pointers is overwritten
//! ```
//!
//! Currently supported platforms are:
//! - `x86_64-pc-windows-msvc`
//! - `x86_64-unknown-linux-gnu`
//!
//! At the moment, cross-compilation is not possible.
//!
//! This library does not distribute `CTimeTagLib.lib` (Windows) or
//! `libtimetag64.so` (Linux). Please source them from the vendor and
//! place them in `timetag/lib/`.
//!
//! This library is intended to be used with a separate user interface
//! or server implementation: `timetag` only provides the Rust FFI to
//! the vendor library.

use std::collections::HashSet;

/// [CXX](https://cxx.rs) interface to vendor's C++ library.
///
/// Because the vendor library is a C-style raw pointer interface, it
/// is wrapped in another class that provides a level of indirection
/// to these objects, in the "modern" smart-pointer style that is
/// required by CXX.
///
/// ## Note: FFI integer types
///
/// The CTimeTag library uses `long long` as the time type
/// and for certain counters and `int`s as bitmask values for 16-channel
/// patterns, input/output channel numbers, windows and delays, and
/// certain counters.
///
/// In all cases, the 2's complement interpretation of the MSB is
/// irrelevant, as in the case of the timestamp the MSB is never
/// reached under normal operation, and for the others, the value is
/// only a partial width, e.g. 18 or 28 bits, or takes a limited range,
/// like input/output channels.
///
/// Because the FFI requires that the implementation-dependent types be
/// cast to fixed-width types anyway, almost all integers are cast to
/// unsigned to follow their semantic meaning. 64 and 32 bit counters,
/// and 32-bit masks keep their bit depth, but channels are consistently
/// reduced to a `u8`, and certain variables with limited-range arguments
/// like those taken by `set_led_brightness()` and `set_output_width()`
/// are also changed to `u8`. An exception is the tag timestamps, where
/// in the analysis it is very common to take differences between tags,
/// so they are left as `i64`s in the FFI to avoid annoying casting
/// everywhere a negative value may occur.
///
/// Arguments are not checked for valid values, beyond using unsigned
/// integers where appropriate, and restricting to the smallest possible
/// fixed-size integers.
#[cxx::bridge(namespace = "TimeTag")]
pub mod ffi {
    // Shared structs whose fields are visible to both langs

    /// Time tag struct. Because CXX places constraints on what traits
    /// can be derived in structs, we use a special-purpose struct in
    /// the FFI only, then collect into `tagtools::Tag` for any actual
    /// analysis, serialization, etc.
    #[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
    struct FfiTag {
        time: i64,
        channel: u8,
    }

    // C++ types and signatures exposed to Rust
    unsafe extern "C++" {
        include!("timetag/include/taghelper.h");

        // Helper class wrapping vendor's CTimeTag
        type TimeTagger;
        /*
          If only one type is defined in this extern block, the
          signatures with &self are automatically associated as methods
          for that type. For more than one, use multiple extern blocks,
          or specify the type explicitly, e.g. self: &TimeTagger
        */

        // Wrappers for vendor class methods
        fn open(self: &TimeTagger) -> ();
        fn close(self: &TimeTagger) -> ();
        fn calibrate(self: &TimeTagger) -> ();
        fn read_error_flags(self: &TimeTagger) -> u32;
        fn get_no_inputs(self: &TimeTagger) -> u8;
        fn get_resolution(self: &TimeTagger) -> f64;
        fn set_input_threshold(self: &TimeTagger, input: u8, voltage: f64) -> ();
        fn set_inversion_mask(self: &TimeTagger, mask: u32) -> ();
        fn set_delay(self: &TimeTagger, input: u8, delay: u32) -> ();
        fn get_fpga_version(self: &TimeTagger) -> i32;
        fn set_led_brightness(self: &TimeTagger, percent: u8);
        fn set_fg(self: &TimeTagger, period: u32, high: u32) -> ();

        fn freeze_single_counter(self: &TimeTagger) -> u64; //
        fn get_single_count(self: &TimeTagger, input: u8) -> u32;

        fn start_timetags(self: &TimeTagger) -> ();
        fn stop_timetags(self: &TimeTagger) -> ();
        fn read_tags(self: &TimeTagger) -> UniquePtr<CxxVector<FfiTag>>;

        fn use_timetag_gate(self: &TimeTagger, b: bool) -> ();
        fn set_gate_width(self: &TimeTagger, duration: u32) -> ();
        fn use_level_gate(self: &TimeTagger, b: bool) -> ();
        fn level_gate_active(self: &TimeTagger) -> bool;
        fn set_filter_min_count(self: &TimeTagger, min_count: u32) -> ();
        fn set_filter_max_time(self: &TimeTagger, max_time: u32) -> ();
        fn set_filter_exception(self: &TimeTagger, exception: u32) -> ();
        fn use_10MHz(self: &TimeTagger, b: bool) -> ();

        // Wrappers for helper functions
        fn new_time_tagger() -> UniquePtr<TimeTagger>;

        // Helper class wrapping vendor's CLogic
        type LogicCounter;

        // Wrappers for vendor class methods
        fn switch_logic_mode(self: &LogicCounter) -> ();
        fn set_window_width(self: &LogicCounter, window: u32) -> ();
        fn set_delay(self: &LogicCounter, input: u8, delay: u32) -> ();
        fn read_logic(self: &LogicCounter) -> i64; // return value is debug only
        fn calc_count(self: &LogicCounter, pos: u16, neg: u16) -> u32;
        fn calc_count_pos(self: &LogicCounter, pos: u16) -> u32;
        fn get_time_counter(self: &LogicCounter) -> u64;
        fn set_output_width(self: &LogicCounter, width: u8) -> ();
        fn set_output_pattern(self: &LogicCounter, output: u8, pos: u16, neg: u16) -> ();
        fn set_output_event_count(self: &LogicCounter, events: u32) -> ();
        // Inner time_tagger methods
        fn open(self: &LogicCounter) -> ();
        fn close(self: &LogicCounter) -> ();
        fn calibrate(self: &LogicCounter) -> ();
        fn read_error_flags(self: &LogicCounter) -> u32;
        fn get_no_inputs(self: &LogicCounter) -> u8;
        fn get_resolution(self: &LogicCounter) -> f64;
        fn set_input_threshold(self: &LogicCounter, input: u8, voltage: f64) -> ();
        fn set_inversion_mask(self: &LogicCounter, mask: u32) -> ();
        // set_delay removed
        fn get_fpga_version(self: &LogicCounter) -> i32; // return value is debug only
        fn set_led_brightness(self: &LogicCounter, percent: u8) -> ();
        fn set_fg(self: &LogicCounter, period: u32, high: u32) -> ();
        // singles counter methods removed
        // timetag methods removed
        fn use_10MHz(self: &LogicCounter, b: bool) -> ();

        fn new_logic_counter() -> UniquePtr<LogicCounter>;
    }
}

pub fn error_text(flags: u32) -> HashSet<String> {
    let mut e: HashSet<String> = HashSet::new();
    let mut f = flags;
    loop {
        match f.trailing_zeros() {
            0 => {
                e.insert(String::from("DataOverflow"));
                f ^= 1 << 0;
            }
            1 => {
                e.insert(String::from("NegFifoOverflow"));
                f ^= 1 << 1;
            }
            2 => {
                e.insert(String::from("PosFifoOverflow"));
                f ^= 1 << 2;
            }
            3 => {
                e.insert(String::from("DoubleError"));
                f ^= 1 << 3;
            }
            4 => {
                e.insert(String::from("InputFifoOverflow"));
                f ^= 1 << 4;
            }
            5 => {
                e.insert(String::from("10MHzHardError"));
                f ^= 1 << 5;
            }
            6 => {
                e.insert(String::from("10MHzSoftError"));
                f ^= 1 << 6;
            }
            7 => {
                e.insert(String::from("OutFifoOverflow"));
                f ^= 1 << 7;
            }
            8 => {
                e.insert(String::from("OutDoublePulse"));
                f ^= 1 << 8;
            }
            9 => {
                e.insert(String::from("OutTooLate"));
                f ^= 1 << 9;
            }
            28 => {
                e.insert(String::from("OutOfSequence"));
                f ^= 1 << 28;
            }
            32 => break,
            x => {
                e.insert(format!("UnknownFlag{}", x));
                f ^= 1 << x;
            }
        }
    }
    return e;
}
