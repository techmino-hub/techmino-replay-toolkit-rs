//! Test streaming by generating replays that should have a significant impact on RAM if directly
//! and fully stored there.
//!
//! The deserialized output is then checked by comparing the `GameInputEvent` stream that we expect
//! utilizing a previously-cloned rng instance.

use fastrand::Rng;
use libtechmino_replay::{
    deserialize::ReplayDecoder, format::ReplayBufferKind, serialize::ReplayEncoder, *,
};

use crate::common::{
    SAMPLE_METADATA, STREAMING_INPUTS_PER_ROUND, generate_event_chunk, random_action,
    random_game_input_event,
};

pub mod common;

#[test]
fn test_streaming() {
    let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

    let round_counts = [2, 4, 8];
    let input_modes = [InputParseMode::Absolute, InputParseMode::Relative];
    let replay_kinds = [
        ReplayBufferKind::Uncompressed,
        ReplayBufferKind::Compressed,
        ReplayBufferKind::Base64,
    ];

    for rbk in replay_kinds {
        for input_mode in input_modes {
            for round_count in round_counts {
                test_streaming_with_params(round_count, input_mode, rbk, &mut rng);
            }
        }
    }
}

#[test]
#[ignore = "this takes a really long time"]
fn test_streaming_extended() {
    let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

    let round_counts = [2, 4, 8, 16, 32, 128, 256, 512, 1024, 2048];
    let input_modes = [InputParseMode::Absolute, InputParseMode::Relative];
    let replay_kinds = [
        ReplayBufferKind::Uncompressed,
        ReplayBufferKind::Compressed,
        ReplayBufferKind::Base64,
    ];

    for rbk in replay_kinds {
        for input_mode in input_modes {
            for round_count in round_counts {
                test_streaming_with_params(round_count, input_mode, rbk, &mut rng);
            }
        }
    }
}

struct InputGenState {
    rng: Rng,
    prev_input_frame: u64,
}

/// Streaming test for a certain round count parameter.
///
/// `input_rng` is the rng instance used for generating the game input event sequence.
fn test_streaming_with_params(
    round_count: usize,
    input_mode: InputParseMode,
    replay_kind: ReplayBufferKind,
    input_rng: &mut Rng,
) {
    eprintln!(
        "test params: {replay_kind:?}, {input_mode:?}, {round_count} rounds × {STREAMING_INPUTS_PER_ROUND} inputs each = {total_inputs} inputs.",
        total_inputs = STREAMING_INPUTS_PER_ROUND * round_count
    );

    let mut encoder = ReplayEncoder::new(replay_kind, 1);
    let metadata_bytes = encoder
        .feed_metadata(&SAMPLE_METADATA, Some(input_mode))
        .expect("feeding metadata should succeed");

    let mut decoder = ReplayDecoder::new(replay_kind, Some(input_mode));

    let mut generator_state = InputGenState {
        prev_input_frame: 0,
        rng: input_rng.fork(),
    };

    let mut checker_state = InputGenState {
        prev_input_frame: 0,
        rng: generator_state.rng.clone(),
    };

    // Temp buffer to be fed to serialization
    let mut this_frame_input_data = Vec::with_capacity(STREAMING_INPUTS_PER_ROUND);
    // Temp buffer from serialization to be fed into deser
    let mut ser_out_buf = metadata_bytes;
    // Accumulative length of serialization output
    let mut ser_out_acc_len = 0;

    for _ in 0..round_count {
        generate_event_chunk(
            &mut generator_state.rng,
            &mut generator_state.prev_input_frame,
            &mut this_frame_input_data,
        );

        encoder
            .feed_input_data(&this_frame_input_data, &mut ser_out_buf)
            .expect("feeding input data should succeed");

        if ser_out_buf.is_empty() {
            continue;
        }

        ser_out_acc_len += ser_out_buf.len();

        let deser_output = decoder
            .update(ser_out_buf.drain(..).as_slice())
            .expect("decoding should work");

        for game_input in deser_output.inputs {
            let expected_input = random_game_input_event(
                &mut checker_state.rng,
                &mut checker_state.prev_input_frame,
            );
            assert_eq!(game_input, expected_input);
        }
    }

    encoder
        .finish(&mut ser_out_buf)
        .expect("encoder should finish");

    ser_out_acc_len += ser_out_buf.len();

    let deser_output = decoder
        .update(ser_out_buf.drain(..).as_slice())
        .expect("decoder should properly finish deserializing replay");

    for game_input in deser_output.inputs {
        let action = random_action(&mut checker_state.rng);
        let frame = checker_state.prev_input_frame + checker_state.rng.u64(0..4);
        checker_state.prev_input_frame = frame;

        let expected_input =
            GameInputEvent::new(frame, action).expect("frame number should be within bounds");

        assert_eq!(game_input, expected_input);
    }

    assert!(decoder.is_finished());

    eprintln!("serializer outputted {ser_out_acc_len} bytes in total");
}
