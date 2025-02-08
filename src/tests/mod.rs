mod cases; use std::fs;

use cases::*;
use ron::ser::PrettyConfig;

use crate::GameReplayData;

#[test]
fn test_serialize_deserialize_noop() {
    let cases = get_test_cases();

    for (key, val) in cases {
        let data = match val.data {
            Some(d) => d,
            None => {
                println!("Skipping testcase '{key}' (it has no deserialized data form)");
                continue;
            },
        };

        println!("Testing for testcase {key}");        

        let serialized = data.serialize_to_raw(None)
            .expect("Error while serializing replay");

        let deserialized = GameReplayData::try_from_raw(&serialized, None)
            .expect("Error while deserializing replay");

        assert_eq!(data, deserialized, "Original and deserialized data doesn't match up!");
    }
}

#[test]
fn test_deserialize_serialize_noop() {
    let cases = get_test_cases();

    for (key, val) in cases {
        let serialized = match val.serialized {
            Some(r) => r,
            None => {
                println!("Skipping testcase '{key}' (it has no serialized data form)");
                continue;
            },
        };

        println!("Testing for testcase {key}");
        
        let deserialized = match serialized {
            StoredReplay::Base64(ref data) => GameReplayData::try_from_base64(data, None),
            StoredReplay::Binary(ref data) => GameReplayData::try_from_compressed(data, None),
        }.expect("Failed to deserialize data");

        let reserialized = match serialized {
            StoredReplay::Base64(_) => StoredReplay::Base64(
                deserialized.serialize_to_base64(None)
                    .expect("Failed to reserialize data")
            ),
            StoredReplay::Binary(_) => StoredReplay::Binary(
                deserialized.serialize_to_compressed(None)
                    .expect("Failed to reserialize data")
                    .into_boxed_slice()
            ),
        };

        assert_eq!(serialized, reserialized, "Original and reserialized form doesn't match!");
    }
}

#[test]
fn test_difference() {
    // TODO:
    // Check if there is a difference between parsed replay and the one gotten from the RON
}

fn get_ron_config() -> PrettyConfig {
    PrettyConfig::new()
        .struct_names(true)
}

#[test]
#[ignore =
    "This test is only for regenerating test cases.\
    Run with 'cargo test regenerate_cases -- --ignored'"]
fn regenerate_cases() {
    let cases = get_test_cases();

    let ron_config = get_ron_config();

    for (key, val) in cases {
        if val.serialized.is_none() { continue; }

        let res = match val.serialized.unwrap() {
            StoredReplay::Base64(string) => GameReplayData::try_from_base64(&string, None),
            StoredReplay::Binary(bytes) => GameReplayData::try_from_compressed(&bytes, None),
        };

        println!("==========[ {key} ]==========\n\n");

        if let Err(e) = res {
            println!("Error parsing replay: {e:?}\n\n");
            continue;
        }

        let res = res.unwrap();

        let ron = ron::ser::to_string_pretty(&res, ron_config.clone());

        let ron = match ron {
            Ok(r) => r,
            Err(e) => {
                println!("Error converting to pretty RON: {e:?}");
                continue;
            }
        };

        if ron.len() > 65536 {
            println!("...{} bytes of RON", ron.len());

            let final_ten = &res.inputs[res.inputs.len() - 11..];

            println!("Final inputs:\n{final_ten:?}");
        } else {
            println!("{ron}\n\n");
        }

        let file_path = &format!(
            "{root}/{key}.ron",
            root = cases::TESTCASE_PATH
        );

        match fs::write(file_path, ron) {
            Ok(_) => {
                println!("Successfully written RON to {file_path}");
            },
            Err(e) => {
                println!("Error while writing RON to '{file_path}': {e}");
            }
        }
    }
}