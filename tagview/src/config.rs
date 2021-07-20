use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct RunConfig {
    pub name:              String,
    pub time_limit:        Option<u32>,
    pub singles_limit:     Option<(u8, u64)>,
    pub coincidence_limit: Option<(u8, u8, u64)>,
    pub save_counts:       Option<bool>,
    pub save_tags:         Option<bool>,
    #[serde(default = "emptyvec", skip_serializing_if = "Vec::is_empty")]
    pub singles:           Vec<u8>,
    #[serde(default = "emptyvec", skip_serializing_if = "Vec::is_empty")]
    pub coincidences:      Vec<(u8, u8)>,
    #[serde(default = "emptyvec", skip_serializing_if = "Vec::is_empty")]
    pub channel_settings:  Vec<ChannelSettings>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ChannelSettings {
    pub channel: u8,
    pub invert: Option<bool>,
    pub delay: Option<u32>,
    pub threshold: Option<f64>,
}

fn emptyvec<T>() -> Vec<T> {
    Vec::new()
}

impl Default for RunConfig {
    fn default() -> Self {
        RunConfig {
            name:              String::new(),
            time_limit:        None,
            singles_limit:     None,
            coincidence_limit: None,
            save_counts:       None,
            save_tags:         None,
            singles:           Vec::new(),
            coincidences:      Vec::new(),
            channel_settings:  Vec::new(),
        }
    }
}