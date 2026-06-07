mod cases;

extern crate std;
use crate::{
    format::ReplayBufferKind, vlq::VlqData, GameInputEvent, GameReplayData, GameReplayMetadata,
    InputAction, InputActionKey, InputActionKind, PlayerSettings,
};
use cases::*;
use core::ops::Deref;
use fastrand::Rng;
use ron::ser::PrettyConfig;
use std::{collections::HashMap, format, fs, println, sync::LazyLock};

const TEST_DATA_UNCOMPRESSED_LEN: usize = 16384;
pub const TEST_CHUNK_MAX_SIZE: usize = 48;

pub static SAMPLE_METADATA: LazyLock<GameReplayMetadata> = LazyLock::new(|| GameReplayMetadata {
    date: String::from("2026-01-01 01:23:45"),
    mode: String::from("sprint_10l"),
    mods: None,
    player: String::from("Stacker"),
    nonstandard: HashMap::new(),
    private: None,
    seed: 0,
    setting: PLAYER_SETTINGS.clone(),
    tas_used: Some(false),
    version: String::from("V0.17.21"),
});

pub static PLAYER_SETTINGS: LazyLock<PlayerSettings> = LazyLock::new(|| PlayerSettings {
    das: Some(4),
    arr: Some(0),
    atk_fx: Some(0),
    bag_line: Some(true),
    block: Some(true),
    center: Some(1.0),
    clear_fx: Some(0),
    dascut: Some(2),
    drop_fx: Some(0),
    dropcut: Some(0),
    face: Some(vec![0; 29]),
    ft_lock: None,
    ghost: Some(0.8),
    grid: Some(0.5),
    high_cam: Some(true),
    ihs: Some(true),
    ims: Some(true),
    irs: Some(true),
    irscut: Some(2),
    lock_fx: Some(1),
    move_fx: Some(0),
    next_pos: Some(true),
    nonstandard: HashMap::new(),
    rs: Some(String::from("TRS")),
    score: Some(true),
    sdarr: Some(0),
    sddas: Some(0),
    shake_fx: Some(0),
    skin: Some(vec![
        1, 7, 11, 3, 14, 4, 9, 1, 7, 2, 6, 10, 2, 13, 5, 9, 15, 4, 11, 3, 12, 2, 16, 8, 4, 10, 13,
        2, 8,
    ]),
    smooth: Some(true),
    splash_fx: Some(0),
    text: Some(true),
    warn: Some(true),
});

macro_rules! const_unwrap_result {
    ($e:expr $(,)*) => {
        match $e {
            ::core::result::Result::Ok(x) => x,
            ::core::result::Result::Err(_) => {
                panic!("attempt to unwrap an err");
            }
        }
    };
}

pub static SAMPLE_INPUT_DATA: [GameInputEvent; 2] = [
    const_unwrap_result!(GameInputEvent::new(
        1,
        InputAction {
            kind: InputActionKind::Press,
            key: InputActionKey::MoveLeft,
        },
    )),
    const_unwrap_result!(GameInputEvent::new(
        9,
        InputAction {
            kind: InputActionKind::Release,
            key: InputActionKey::MoveLeft,
        },
    )),
];

pub static SAMPLE_UNSORTED_INPUT_DATA: [GameInputEvent; 3] = [
    const_unwrap_result!(GameInputEvent::new(
        1,
        InputAction {
            kind: InputActionKind::Press,
            key: InputActionKey::MoveLeft,
        },
    )),
    const_unwrap_result!(GameInputEvent::new(
        9,
        InputAction {
            kind: InputActionKind::Release,
            key: InputActionKey::MoveLeft,
        },
    )),
    const_unwrap_result!(GameInputEvent::new(
        3,
        InputAction {
            kind: InputActionKind::Press,
            key: InputActionKey::MoveLeft,
        },
    )),
];

/// Creates not-quite-random data.
pub(crate) fn slightly_random_data(rng: &mut Rng) -> [u8; TEST_DATA_UNCOMPRESSED_LEN] {
    let test_data_bit_per_byte = rng.usize(0..3);

    core::array::from_fn::<u8, TEST_DATA_UNCOMPRESSED_LEN, _>(|_| {
        // For every byte, choose at most 3 random bits to turn on
        let mut byte = 0;

        for _ in 0..test_data_bit_per_byte {
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
                GameReplayData::parse_replay(string.as_bytes(), ReplayBufferKind::Base64, None)
            }
            StoredReplay::Binary(bytes) => {
                GameReplayData::parse_replay(&bytes, ReplayBufferKind::Compressed, None)
            }
        };

        println!("==========[ {key} ]==========\n\n");

        if let Err(e) = res {
            println!("Error parsing replay: {e:?}\n\n");
            continue;
        }

        let res = res.unwrap();

        let ron = ron::ser::to_string_pretty(&res, ron_config.clone());

        //
        // // Hello this is a wakatime test for testing purposes if you're seeing this please ignore
        // Hello this is a wakatime test for testing purposes if you're seeing this please ignore

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
