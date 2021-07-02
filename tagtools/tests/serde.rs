use std::vec::Vec;
use tagtools::Tag;
use tagtools::{ser, de};

/// Serialize and deserialize tags written as one message to the buffer
#[test]
fn serde_one_message() {
    let tags= vec![
        Tag { time:  0, channel: 1 },
        Tag { time:  6, channel: 2 },
        Tag { time: 12, channel: 1 },
        Tag { time: 18, channel: 2 },
        Tag { time: 24, channel: 1 },
        Tag { time: 30, channel: 2 },
    ];

    let mut b: Vec<u8> = Vec::new();

    ser::tags(&mut b, &tags).unwrap();

    let tags2 = de::tags(&*b).unwrap();

    assert_eq!(&tags, &tags2);
}

/// Serialize and deserialize tags written as multiple messages to the buffer
#[test]
fn serde_many_messages() {
    let tags= vec![
        Tag { time:  0, channel: 1 },
        Tag { time:  6, channel: 2 },
        Tag { time: 12, channel: 1 },
        Tag { time: 18, channel: 2 },
        Tag { time: 24, channel: 1 },
        Tag { time: 30, channel: 2 },
    ];

    let mut b: Vec<u8> = Vec::new();

    for &tag in &tags {
        ser::tags(&mut b, &[tag]).unwrap();
    }

    let tags2 = de::tags(&*b).unwrap();

    assert_eq!(&tags, &tags2);
}