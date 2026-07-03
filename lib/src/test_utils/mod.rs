//! Utilities for unit tests and integration tests.
#![allow(
    clippy::std_instead_of_alloc,
    reason = "tests aren't meant to be run no_std"
)]
#![allow(
    clippy::std_instead_of_core,
    reason = "tests aren't meant to be run no_std"
)]
#![allow(
    dead_code,
    reason = "this module is being imported in both unit tests and integration tests"
)]
#![allow(
    clippy::missing_panics_doc,
    reason = "test utils module isn't meant to be panic-free"
)]

extern crate std;
use crate::{
    GameInputEvent, GameReplayMetadata, InputAction, InputActionKey, InputActionKind,
    PlayerSettings,
};
#[allow(
    unused_imports,
    reason = "this module is being imported in both unit tests and integration tests"
)]
use cases::*;
use core::ops::Deref;
#[cfg(test)]
use fastrand::Rng;
#[cfg(test)]
use ron::ser::PrettyConfig;
use std::sync::LazyLock;

pub mod cases;

/// How many inputs to insert into the stream every round in the `test_streaming`
/// test.
pub const STREAMING_INPUTS_PER_ROUND: usize = 1_048_576;

pub const TEST_DATA_UNCOMPRESSED_LEN: usize = 65536;
pub const TEST_CHUNK_MAX_SIZE: usize = 48;

pub static SAMPLE_METADATA: LazyLock<GameReplayMetadata> = LazyLock::new(|| {
    let mut metadata = GameReplayMetadata::new();

    metadata.set_date("2026-01-01 01:23:45".into());
    metadata.set_mode("sprint_10l".into());
    metadata.set_mods(Some(Vec::new()));
    metadata.set_player("Stacker".into());
    metadata.set_seed(0.into());
    metadata.set_settings(Some(PLAYER_SETTINGS.clone().map));
    metadata.set_tas_used(Some(false));
    metadata.set_version("V0.17.21".into());

    metadata
});

pub static PLAYER_SETTINGS: LazyLock<PlayerSettings> = LazyLock::new(|| {
    let mut settings = PlayerSettings::new();

    settings.set_das(Some(4));
    settings.set_arr(Some(0));
    settings.set_atk_fx(Some(0));
    settings.set_bag_line(Some(true));
    settings.set_block(Some(true));
    settings.set_center(Some(1.0));
    settings.set_clear_fx(Some(0));
    settings.set_dascut(Some(2));
    settings.set_drop_fx(Some(0));
    settings.set_dropcut(Some(0));
    settings.set_face(Some([0; 29]));
    settings.set_ft_lock(None);
    settings.set_ghost(Some(0.8));
    settings.set_grid(Some(0.5));
    settings.set_high_cam(Some(true));
    settings.set_ihs(Some(true));
    settings.set_ims(Some(true));
    settings.set_irs(Some(true));
    settings.set_irscut(Some(2));
    settings.set_lock_fx(Some(1));
    settings.set_move_fx(Some(0));
    settings.set_next_pos(Some(true));
    settings.set_rs(Some("TRS"));
    settings.set_score(Some(true));
    settings.set_sdarr(Some(0));
    settings.set_sddas(Some(0));
    settings.set_shake_fx(Some(0));
    settings.set_skin(Some([
        1, 7, 11, 3, 14, 4, 9, 1, 7, 2, 6, 10, 2, 13, 5, 9, 15, 4, 11, 3, 12, 2, 16, 8, 4, 10, 13,
        2, 8,
    ]));
    settings.set_smooth(Some(true));
    settings.set_splash_fx(Some(0));
    settings.set_text(Some(true));
    settings.set_warn(Some(true));

    settings
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
pub fn slightly_random_data(rng: &mut Rng) -> Box<[u8]> {
    let test_data_bit_per_byte = rng.usize(0..3);

    (0..TEST_DATA_UNCOMPRESSED_LEN)
        .map(|_| {
            // For every byte, choose at most 3 random bits to turn on
            let mut byte = 0;

            for _ in 0..test_data_bit_per_byte {
                let bit = rng.u8(..8);

                let mask = 1u8 << bit;

                byte |= mask;
            }

            byte
        })
        .collect()
}

/// A struct to split an input data into randomly-sized chunks.
pub struct ByteFeeder<'a> {
    /// The slice representing the yet-to-be-output data.
    data: &'a [u8],
}

impl<'a> ByteFeeder<'a> {
    /// Creates a new byte feeder.
    #[must_use]
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Get a randomly-sized chunk of data.
    #[must_use]
    pub fn bite(&mut self, rng: &mut Rng) -> &'a [u8] {
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

/// Test for the [`ByteFeeder`] test util
#[test]
fn test_byte_feeder() {
    let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

    for _ in 0..1024 {
        let init_data = slightly_random_data(&mut rng);
        let mut feeder = ByteFeeder::new(&init_data);
        let mut feeder_output = Vec::with_capacity(TEST_DATA_UNCOMPRESSED_LEN);

        while !feeder.is_empty() {
            feeder_output.extend_from_slice(feeder.bite(&mut rng));
        }

        assert_eq!(&*init_data, feeder_output.as_slice());
    }
}

fn get_ron_config() -> PrettyConfig {
    PrettyConfig::new().struct_names(true)
}

#[cfg(all(test, not(feature = "preserve_metadata_order")))]
#[test]
#[ignore = "This test is only for regenerating test cases.\
    Run with `cargo test regenerate_cases --features preserve_metadata_order -- --ignored`"]
fn regenerate_cases() {
    panic!("metadata order should be preserved when creating initial RONs");
}

#[cfg(all(test, feature = "preserve_metadata_order"))]
#[test]
#[ignore = "This test is only for regenerating test cases.\
    Run with `cargo test regenerate_cases --features preserve_metadata_order -- --ignored`"]
fn regenerate_cases() {
    use crate::{format::ReplayBufferKind, GameReplayData};
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

        match std::fs::write(file_path, ron) {
            Ok(()) => {
                println!("Successfully written RON to {file_path}");
            }
            Err(e) => {
                println!("Error while writing RON to '{file_path}': {e}");
            }
        }
    }
}

pub fn random_action(rng: &mut Rng) -> InputAction {
    let bool = rng.bool();
    let kind = InputActionKind::from_bool(bool);

    let key = rng.u8(InputActionKey::MoveLeft as u8..=InputActionKey::RightZangi as u8);

    // SAFETY: The range MoveLeft..=RightZangi is contiguous and is always valid.
    let key = unsafe { InputActionKey::try_from_byte(key).unwrap_unchecked() };

    InputAction { kind, key }
}

pub fn random_game_input_event(rng: &mut Rng, prev_input_frame: &mut u64) -> GameInputEvent {
    let action = random_action(rng);
    let frame = (*prev_input_frame + rng.u64(0..4)).min(GameInputEvent::MAX_FRAME);
    *prev_input_frame = frame;

    // SAFETY: frame is clamped to be at most the maximum frame GameInputEvent can handle.
    unsafe { GameInputEvent::new(frame, action).unwrap_unchecked() }
}

/// Overwrites a vec with an event chunk consisting of many events.
pub fn generate_event_chunk(
    rng: &mut Rng,
    prev_input_frame: &mut u64,
    out_vec: &mut Vec<GameInputEvent>,
) {
    out_vec.clear();

    out_vec.reserve_exact(STREAMING_INPUTS_PER_ROUND);

    for _ in 0..STREAMING_INPUTS_PER_ROUND {
        out_vec.push(random_game_input_event(rng, prev_input_frame));
    }
}
