use tagview::settings::{RunConfig, ChannelSettings};

fn main() {
    let x = RunConfig {
        name: String::from("test_settings_serde"),
        time_limit: None,
        singles_limit: None,
        coincidence_limit: None,
        save_counts: Some(false),
        save_tags: None,
        channel_settings: vec![
            ChannelSettings {
                channel: 1,
                invert: None,
                delay: None,
                threshold: None,
            },
            ChannelSettings {
                channel: 2,
                invert: None,
                delay: Some(486),
                threshold: None,
            }
        ],
        singles: vec![1,2,3],
        coincidences: vec![(1, 2), (1,3), (2,3)],
    };

    let ser = toml::ser::to_string(&x).unwrap();

    println!("{}", ser);
}