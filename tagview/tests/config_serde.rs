use tagview::config::{RunConfig, ChannelSettings};

fn serialize_config(config: &RunConfig) -> String {
    let ser = toml::ser::to_string(config).unwrap();
    return ser;
}

fn deserialize_config(config: &str) -> RunConfig {
    let de: RunConfig = toml::de::from_str(config).unwrap();
    return de;
}

#[test]
fn serde_config() {
    let config = RunConfig {
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
    let serconfig = serialize_config(&config);
    let deconfig = deserialize_config(&serconfig);
    assert_eq!(config, deconfig);
}

#[test]
fn simple1() {
    let x =
        "name = \"test_settings_serde\"
        singles = [ 1, 2, 3, 4, 5, 6, 7, 8 ]
        coincidences = [ [1, 2], [2, 3], [3, 4], [4, 5] ]";

    let de: RunConfig = toml::de::from_str(x).unwrap();

    let r = RunConfig {
        name: "test_settings_serde".to_owned(),
        singles: vec![1, 2, 3, 4, 5, 6, 7, 8],
        coincidences: vec![(1,2), (2,3), (3,4), (4,5)],
        ..Default::default()
    };
    
    assert_eq!(r, de);
}

#[test]
fn simple2() {
    let x =
        "name = \"test_config\"
        singles = [ 1, 2, 3, 4 ]
        coincidences = [ [1, 2], [2, 3], [3, 4], [4, 5] ]";
    
    let de: RunConfig = toml::de::from_str(x).unwrap();

    let r = RunConfig {
        name: "test_config".to_owned(),
        singles: vec![1, 2, 3, 4],
        coincidences: vec![(1,2), (2,3), (3,4), (4,5)],
        ..Default::default()
    };
    
    assert_eq!(r, de);
}