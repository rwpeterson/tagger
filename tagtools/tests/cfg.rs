use tagtools::cfg::{ChannelSettings, Coincidence, Run, RunLimit, SaveTags, Single};

fn serialize_config(config: &Run) -> String {
    let ser = serde_json::to_string(config).unwrap();
    return ser;
}

fn deserialize_config(config: &str) -> Run {
    let de: Run = serde_json::from_str(config).unwrap();
    return de;
}

#[test]
fn serde_roundtrip() {
    let config = Run {
        description: String::from("test_settings_serde"),
        version: String::new(),
        limit: None,
        duration: None,
        timestamp: None,
        save_counts: Some(false),
        save_tags: Some(SaveTags::Save(false)),
        tagmask: None,
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
        singles: vec![
            Single::Channel(1),
            Single::Channel(2),
            Single::Channel(3),
        ],
        coincidences: vec![
            Coincidence::Channels((1, 2)),
            Coincidence::Channels((1, 3)),
            Coincidence::Channels((2, 3)),
        ],
    };
    let serconfig = serialize_config(&config);
    let deconfig = deserialize_config(&serconfig);
    assert_eq!(config, deconfig);
}

#[test]
fn de_simple() {
    let x =
        r#"{
            "description": "test_settings_serde",
            "singles": [
                {"channel": 1},
                {"channel": 2},
                {"channel": 3},
                {"channel": 4},
                {"channel": 5},
                {"channel": 6},
                {"channel": 7},
                {"channel": 8}
            ],
            "coincidences": [ 
                {"channels": [1, 2]}, 
                {"channels": [2, 3]}, 
                {"channels": [3, 4]}, 
                {"channels": [4, 5]}
            ]
        }"#;

    let de: Run = serde_json::from_str(x).unwrap();

    let r = Run {
        description: String::from("test_settings_serde"),
        singles: vec![
            Single::Channel(1),
            Single::Channel(2),
            Single::Channel(3),
            Single::Channel(4),
            Single::Channel(5),
            Single::Channel(6),
            Single::Channel(7),
            Single::Channel(8),
        ],
        coincidences: vec![
            Coincidence::Channels((1, 2)), 
            Coincidence::Channels((2, 3)), 
            Coincidence::Channels((3, 4)), 
            Coincidence::Channels((4, 5)),
        ],
        ..Default::default()
    };
    
    assert_eq!(r, de);
}

#[test]
fn de_complex() {
    let x =
        r#"{
            "description": "test_settings_serde",
            "limit": {"duration": "5 sec"},
            "singles": [
                {"channel": 1},
                {"channel": 2}
            ],
            "coincidences": [ 
                {"channels": [1, 2]}
            ],
            "channel_settings": [
                {"channel": 1, "delay": 0, "threshold": 2.0},
                {"channel": 2, "invert": true, "delay": 42069, "threshold": 1.2}
            ]
        }"#;
    
    let de: Run = serde_json::from_str(x).unwrap();

    let r = Run {
        description: String::from("test_settings_serde"),
        limit: Some(RunLimit::Duration("5 sec".parse::<humantime::Duration>().unwrap().into())),
        singles: vec![
            Single::Channel(1), 
            Single::Channel(2),
        ],
        coincidences: vec![
            Coincidence::Channels((1,2)),
        ],
        channel_settings: vec![
            ChannelSettings { channel: 1, invert: None, delay: Some(0), threshold: Some(2.0) },
            ChannelSettings { channel: 2, invert: Some(true), delay: Some(42069), threshold: Some(1.2) },
        ],
        ..Default::default()
    };

    assert_eq!(r, de);
}