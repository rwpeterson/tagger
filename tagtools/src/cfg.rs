//! Configuration tools: formats for declaring and recording data

use chrono::{DateTime, offset::Local};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::Duration;

/// Experiment run specification for both declaring and recording runs in text files.
/// For concreteness, we use [TOML](https://toml.io) as the text file format.
///
/// ## Declaring a run
///
/// A `.toml` file specifies the data to be recorded. All fields in `Run` are optional:
/// specify only what makes sense. The `name` field is free; set it to a useful value
/// to help keep track of what was done. Beyond that, a minimal specification sets a
/// limit (in user-readable time duration or a number of counts in some pattern),
/// flags whether tags or counts should be saved, and sets the appropriate patterns.
///
/// ## Recording a run
///
/// ### Logic mode
///
/// A run is recorded in the same format as the declation, either by switching enum
/// variants or filling in fields that were empty in the specification. For example,
/// the `singles` field is mapped from `Single::Channel(chan)` to
/// `Single::ChannelCounts(chan, counts)`. The precise duration (in 5 ns
/// increments) is recorded as an integer, leaving rates to be calculated in post.
/// A timestamp of the run start is included for reference, along with the name
/// string provided in the declaration. All channel settings are also recorded.
///
/// ### Tag mode
///
/// Currently, this only looks at `save_tags`: if true, it will save all tags to a file
/// specified in SaveTags::TagFile, whose filename is implementation-dependent.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Run {
    pub name:               String,
    pub timestamp:          Option<DateTime<Local>>,
    pub limit:              Option<RunLimit>,
    pub save_counts:        Option<bool>,
    pub save_tags:          Option<SaveTags>,
    pub duration:           Option<u64>,
    #[serde(default = "emptyvec", skip_serializing_if = "Vec::is_empty")]
    pub singles:            Vec<Single>,
    #[serde(default = "emptyvec", skip_serializing_if = "Vec::is_empty")]
    pub coincidences:       Vec<Coincidence>,
    #[serde(default = "emptyvec", skip_serializing_if = "Vec::is_empty")]
    pub channel_settings:   Vec<ChannelSettings>,
}

/// Either a fixed time duration or limit on some number of a specific pattern.
/// Duration is parsed as in [humantime](https://docs.rs/humantime/), e.g.
/// `15days 2min 2s` or `2years 2min 12us`.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum RunLimit {
    #[serde(with = "humantime_serde")]
    Duration(Duration),
    SinglesLimit(u8, u64),
    CoincidenceLimit(u8, u8, u32, u64),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum SaveTags {
    Save(bool),
    TagFile(PathBuf),
}

/// Specify a channel, or specify a channel with some number of counts
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Single {
    Channel(u8),
    ChannelCounts((u8, u64)),
}

/// Specify two channels, two and a window, or two and a window and counts
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Coincidence {
    Channels((u8, u8)),
    ChannelsWin((u8, u8, u32)),
    ChannelsCounts((u8, u8, u32, u64)),
}


/// All tagger-controlled settings for a given channel
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ChannelSettings {
    pub channel:    u8,
    pub invert:     Option<bool>,
    pub delay:      Option<u32>,
    pub threshold:  Option<f64>,
}

fn emptyvec<T>() -> Vec<T> {
    Vec::new()
}

/// Creates an empty Run. Specific defaults should be implementation-dependent.
impl Default for Run {
    fn default() -> Self {
        Run {
            name:               String::new(),
            timestamp:          None,
            limit:              None,
            save_counts:        None,
            save_tags:          None,
            duration:           None,
            singles:            Vec::new(),
            coincidences:       Vec::new(),
            channel_settings:   Vec::new(),
        }
    }
}