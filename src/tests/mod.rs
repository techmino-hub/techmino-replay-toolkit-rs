mod cases;

extern crate std;
use core::ops::Deref;
use std::{format, fs, println};

use cases::*;
use fastrand::Rng;
use ron::ser::PrettyConfig;

use crate::{format::ReplayBufferKind, vlq::VlqData, GameReplayData};

const TEST_DATA_UNCOMPRESSED_LEN: usize = 16384;
const TEST_CHUNK_MAX_SIZE: usize = 48;

/// Creates not-quite-random data.
pub(crate) fn slightly_random_data(rng: &mut Rng) -> [u8; TEST_DATA_UNCOMPRESSED_LEN] {
    /// At max, how many bits in a byte to turn on.
    const TEST_DATA_BIT_PER_BYTE: usize = 2;

    core::array::from_fn::<u8, TEST_DATA_UNCOMPRESSED_LEN, _>(|_| {
        // For every byte, choose at most 3 random bits to turn on
        let mut byte = 0;

        for _ in 0..TEST_DATA_BIT_PER_BYTE {
            let bit = rng.u8(..8);

            let mask = 1u8 << bit;

            byte |= mask;
        }

        byte
    })
}

pub(crate) fn random_vlq(rng: &mut Rng) -> VlqData {
    let max = match rng.u8(..4) {
        0 => 2,
        1 => const { u8::MAX as u64 },
        2 => const { u16::MAX as u64 },
        _ => VlqData::MAX_REPRESENTABLE,
    };

    let num = rng.u64(..=max);
    VlqData::try_from(num).unwrap()
}

/// A struct to split an input data into randomly-sized chunks.
pub(crate) struct ByteFeeder<'a> {
    /// The slice representing the yet-to-be-output data.
    data: &'a [u8],
}

impl<'a> ByteFeeder<'a> {
    /// Creates a new byte feeder.
    #[must_use]
    pub(crate) const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Get a randomly-sized chunk of data.
    #[must_use]
    pub(crate) fn bite(&mut self, rng: &mut Rng) -> &'a [u8] {
        let chunk_size = rng.usize(1..=(TEST_CHUNK_MAX_SIZE.min(self.data.len())));
        let (chunk, rest) = self.data.split_at(chunk_size);
        self.data = rest;
        chunk
    }
}

impl<'a> Deref for ByteFeeder<'a> {
    type Target = &'a [u8];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[test]
fn internal_test_byte_feeder() {
    let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

    for _ in 0..1024 {
        let init_data = slightly_random_data(&mut rng);
        let mut feeder = ByteFeeder::new(init_data.as_slice());
        let mut feeder_output = Vec::with_capacity(TEST_DATA_UNCOMPRESSED_LEN);

        while !feeder.is_empty() {
            feeder_output.extend_from_slice(feeder.bite(&mut rng));
        }

        assert_eq!(init_data.as_slice(), feeder_output.as_slice());
    }
}

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
            match GameReplayData::parse_replay(&serialized, ReplayBufferKind::Uncompressed) {
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

#[test]
fn test_difference() {
    // TODO:
    // Check if there is a difference between parsed replay and the one gotten from the RON
}

fn get_ron_config() -> PrettyConfig {
    PrettyConfig::new().struct_names(true)
}

#[test]
#[ignore = "This test is only for regenerating test cases.\
    Run with `cargo test regenerate_cases -- --ignored`"]
fn regenerate_cases() {
    let cases = get_test_cases();

    let ron_config = get_ron_config();

    for (key, val) in cases {
        if val.serialized.is_none() {
            continue;
        }

        let res = match val.serialized.unwrap() {
            StoredReplay::Base64(string) => {
                GameReplayData::parse_replay(string.as_bytes(), ReplayBufferKind::Base64)
            }
            StoredReplay::Binary(bytes) => {
                GameReplayData::parse_replay(&bytes, ReplayBufferKind::Compressed)
            }
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

        let file_path = &format!("{root}/{key}.ron", root = cases::TESTCASE_PATH);

        match fs::write(file_path, ron) {
            Ok(()) => {
                println!("Successfully written RON to {file_path}");
            }
            Err(e) => {
                println!("Error while writing RON to '{file_path}': {e}");
            }
        }
    }
}
