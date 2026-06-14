//! `roundtrip_noop`: Asserts that serialization -> deserialization is a no-op.

use libtechmino_replay::{format::ReplayBufferKind, *};

use crate::common::cases::get_test_cases;

pub mod common;

#[test]
fn test_serialize_deserialize_noop() {
    let cases = get_test_cases();

    for (key, val) in cases {
        let Some(data) = val.data else {
            println!("Skipping testcase '{key}' (it has no deserialized data form)");
            continue;
        };

        println!("Testing for testcase {key}");

        let serialized = data
            .serialize_to_raw(None)
            .expect("Error while serializing replay");

        let deserialized =
            match GameReplayData::parse_replay(&serialized, ReplayBufferKind::Uncompressed, None) {
                Ok(r) => r,
                Err(e) => {
                    panic!("Error while deserializing replay {key}: {e:?}");
                }
            };

        // Separate assertions to get more narrow assertion failures
        assert_eq!(
            data.metadata, deserialized.metadata,
            "Original and deserialized metadata doesn't match up!",
        );

        assert_eq!(
            data.inputs, deserialized.inputs,
            "Original and deserialized input data doesn't match up!",
        );

        assert_eq!(
            data, deserialized,
            "Original and deserialized data doesn't match up!"
        );
    }
}
