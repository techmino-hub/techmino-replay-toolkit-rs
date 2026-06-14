//! `deser_data_match`: Asserts that the deserialized replay matches known replay data.

use crate::common::cases::{get_test_cases, StoredReplay};
use libtechmino_replay::*;

mod common;

#[test]
fn test_difference() {
    let cases = get_test_cases();

    for (key, val) in cases {
        let Some(saved_deserialized) = val.data else {
            println!("Skipping testcase '{key}' (it has no deserialized data form)");
            continue;
        };

        let Some(saved_serialized) = val.serialized else {
            println!("Skipping testcase '{key}' (it has no serialized data form)");
            continue;
        };

        println!("Testing for testcase {key}");

        let (saved_serialized_bytes, serialized_kind) = match &saved_serialized {
            StoredReplay::Base64(string) => (string.as_bytes(), ReplayBufferKind::Base64),
            StoredReplay::Binary(binary) => (&binary[..], ReplayBufferKind::Compressed),
        };

        // dbg!(&saved_serialized, &saved_serialized_bytes);

        let parsed = GameReplayData::parse_replay(saved_serialized_bytes, serialized_kind, None)
            .expect("deserialization should work");

        assert_eq!(saved_deserialized, parsed);
    }
}
