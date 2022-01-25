//! Configuration tools: formats for declaring and recording data

use chrono::{DateTime, offset::Utc};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::Duration;

/// Experiment run specification for both declaring and recording runs in text files.
/// We use JSON as the file format. The format is defined via the Rust struct
/// `tagtools::cfg::Run`, with all `PascalCase` enum variants (matching Rust style) renamed
/// to `snake_case` in the JSON for consistency. All fields are formally optional: different
/// subsets are practically required or optional depending on whether the run file specifies
/// data to be taken or is a record of an experiment.
///
/// ## Declaring a run
///
/// A `.json` file specifies the data to be recorded. All fields in `Run` are optional:
/// specify only what makes sense. The `description` field is free, and can contain
/// freeform text. Beyond that, a minimal run file sets a limit (in a user-readable time
/// duration string or a number of counts in some pattern), flags if tags should be saved,
/// and sets the appropriate patterns.
/// Practically, you need to specify channel settings (at least a threshold). Channel
/// settings are **stateful**, so once set they remain in effect until the tagger resets.
/// For this reason implementations should not set channel settings willy-nilly,
/// especially with default values, so that the user only needs to specify them once
/// instead of in every run file. However, implementations should always collect
/// channel settings, to capture a complete record of the experiment.
///
/// ## Recording a run
///
/// ### Logic mode
///
/// A run is recorded in a new file with the same format as the declaration, either
/// by switching enum variants or filling in fields that were empty in the
/// declaration. For example, the contents of `singles` are mapped from
/// `"singles": [{ "channel": 1 }]` to `"singles": [{ "channel": 1, "counts": 12345 }]`,
/// which corresponds to the two Rust enumerants `Single::Channel(u8)` and
/// `Single::ChannelCounts((u8, u64))`. The precise
/// duration (in 5 ns increments) is recorded as an integer, leaving rates to be
/// calculated in post. A timestamp of the run start is included for reference,
/// along with the name string provided in the declaration. All channel settings
/// are also recorded. `myrunfile.json -> <timestamp>-myrunfile.json`, where
/// `<timestamp>` is `%Y%m%dT%H%M%SZ`, e.g. "20220119T123501Z".
///
/// ### Tag mode
///
/// Currently, this only looks at `save_tags`: if true, it will save all tags to a file
/// named `<timestamp>-myrunfile.tags.zst`. This filename is additionally specified in
/// `SaveTags::TagFile` inside `<timestamp>-myrunfile-.json`.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Run {
    pub description:        String,
    #[serde(default = "emptystring", skip_serializing_if = "String::is_empty")]
    pub version:            String,
    pub timestamp:          Option<DateTime<Utc>>,
    pub limit:              Option<RunLimit>,
    pub save_counts:        Option<bool>,
    pub save_tags:          Option<SaveTags>,
    pub tagmask:            Option<u16>,
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
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RunLimit {
    #[serde(with = "humantime_serde")]
    Duration(Duration),
    SinglesLimit(u8, u64),
    CoincidenceLimit(u8, u8, u32, u64),
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SaveTags {
    Save(bool),
    TagFile(PathBuf),
}

/// Specify a channel, or specify a channel with some number of counts
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Single {
    Channel(u8),
    ChannelCounts((u8, u64)),
}

/// Specify two channels, two and a window, or two and a window and counts
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Coincidence {
    Channels((u8, u8)),
    ChannelsWin((u8, u8, u32)),
    ChannelsCounts((u8, u8, u32, u64)),
}


/// All tagger-controlled settings for a given channel
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ChannelSettings {
    pub channel:    u8,
    pub invert:     Option<bool>,
    pub delay:      Option<u32>,
    pub threshold:  Option<f64>,
}

fn emptyvec<T>() -> Vec<T> {
    Vec::new()
}

fn emptystring() -> String {
    String::new()
}

/// Creates an empty Run. Specific defaults should be implementation-dependent.
impl Default for Run {
    fn default() -> Self {
        Run {
            description:        String::new(),
            version:            String::new(),
            timestamp:          None,
            limit:              None,
            save_counts:        None,
            save_tags:          None,
            tagmask:            None,
            duration:           None,
            singles:            Vec::new(),
            coincidences:       Vec::new(),
            channel_settings:   Vec::new(),
        }
    }
}