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

/// Serialize and deserialize 4 GiB of tags (one Cap'n Proto `struct_list` worth)
#[test]
#[ignore]
fn serde_4_gibibytes() {
    let n = 2usize.pow(27);
    let mut tags = Vec::with_capacity(n);
    for i in 0..n {
        tags.push(Tag { time: i as i64, channel: 1 });
    }
    let mut b: Vec<u8> = Vec::new();
    ser::tags(&mut b, &tags).unwrap();
    let tags2 = de::tags(&*b).unwrap();
    assert_eq!(&tags, &tags2);
}

/// Serialize and deserialize 16 GiB of tags (four Cap'n Proto `struct_list`s worth)
#[test]
#[ignore]
fn serde_16_gibibytes() {
    let n = 2usize.pow(29);
    let mut tags = Vec::with_capacity(n);
    for i in 0..n {
        tags.push(Tag { time: i as i64, channel: 1 });
    }
    let mut b: Vec<u8> = Vec::new();
    ser::tags(&mut b, &tags).unwrap();
    let tags2 = de::tags(&*b).unwrap();
    assert_eq!(&tags, &tags2);
}