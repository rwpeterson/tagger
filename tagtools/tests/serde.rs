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

/// Serialize and deserialize just under 4 GiB of tags (one Cap'n Proto `struct_list` worth)
#[test]
#[ignore]
fn serde_one_full_list() {
    let n = (1 << 28) - 1;
    let mut tags = Vec::with_capacity(n);
    for i in 0..n {
        tags.push(Tag { time: i as i64, channel: 1 });
    }
    let mut b: Vec<u8> = Vec::new();
    ser::tags(&mut b, &tags).unwrap();
    let tags2 = de::tags(&*b).unwrap();
    assert_eq!(&tags, &tags2);
}

/// Serialize and deserialize 4 GiB of tags (one full list plus one list with one tag)
#[test]
#[ignore]
fn serde_one_full_list_plus_one() {
    let n = 1 << 28;
    let mut tags = Vec::with_capacity(n);
    for i in 0..n {
        tags.push(Tag {
            time: i as i64,
            channel: 1,
        });
    }
    let mut b: Vec<u8> = Vec::new();
    ser::tags(&mut b, &tags).unwrap();
    let tags2 = de::tags(&*b).unwrap();
    assert_eq!(&tags, &tags2);
}

/// Serialize and deserialize just under 8 GiB of tags (two full lists)
#[test]
#[ignore]
fn serde_two_full_lists() {
    let n = (1 << 29) - 2;
    let mut tags = Vec::with_capacity(n);
    for i in 0..n {
        tags.push(Tag {
            time: i as i64,
            channel: 1,
        });
    }
    let mut b: Vec<u8> = Vec::new();
    ser::tags(&mut b, &tags).unwrap();
    let tags2 = de::tags(&*b).unwrap();
    assert_eq!(&tags, &tags2);
}
