mod cases; use cases::*;
use ron::ser::PrettyConfig;

use crate::GameReplayData;

#[test]
fn test_serialize_deserialize_noop() {
    // TODO: Create testcases with known replays
}

#[test]
fn check_known_cases() {
    // TODO: Use RON to store replay data to compare
}

#[test]
fn regenerate_cases() {
    let cases = get_test_cases();

    for (key, val) in cases {
        if val.serialized.is_none() { continue; }

        let res = match val.serialized.unwrap() {
            StoredReplay::Base64(string) => GameReplayData::try_from_base64(&string, None),
            StoredReplay::Binary(bytes) => GameReplayData::try_from_compressed(&bytes, None),
        };

        println!("==========[ {key} ]==========\n\n");

        if let Err(e) = res {
            println!("{e:?}\n\n");
            continue;
        }

        let res = res.unwrap();

        let ron = ron::ser::to_string_pretty(&res, PrettyConfig::new()).unwrap();

        println!("{ron}\n\n");
    }
}