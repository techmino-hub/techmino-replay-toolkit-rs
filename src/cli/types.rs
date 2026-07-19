use crate::cli::clap::{CliInputMode, CliReplayFormat, ExtractMode, RetryArguments};
use libtechmino_replay::{GameInputEvent, InputAction, InputActionKey, InputActionKind};
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf};
use thiserror::Error;

/// Full argument set for extracting.
pub(super) struct ExtractArguments<'a> {
    pub(super) retry_args: RetryArguments,
    pub(super) extract_mode: &'a ExtractMode,
    pub(super) replay_format: Option<CliReplayFormat>,
    pub(super) override_input_mode: Option<CliInputMode>,
    pub(super) input_file: &'a Option<PathBuf>,
    pub(super) output_json: &'a Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum CliOpError {
    #[error("Failed to open input file at '{path:?}': {inner}")]
    InputStreamOpenError { inner: io::Error, path: PathBuf },
    #[error("Failed to open output file at '{path:?}': {inner}")]
    OutputStreamOpenError { inner: io::Error, path: PathBuf },
    #[error("Failed to infer replay kind; {first_byte} is not a valid first byte")]
    ReplayKindInferFailed { first_byte: u8 },
    #[error("Failed to read from input file: {inner}")]
    InputReadError { inner: io::Error },
    #[error("Failed to write to output file: {inner}")]
    OutputWriteError { inner: io::Error },
    #[error("Failed to flush output stream: {inner}")]
    OutputFlushError { inner: io::Error },
    #[error("Input file is empty")]
    InputEmpty,
    #[error("Failed to parse replay: {inner}")]
    ParseError {
        #[from]
        inner: libtechmino_replay::ReplayParseError,
    },
    #[error("Failed to serialize replay: {inner}")]
    SerializeError {
        #[from]
        inner: libtechmino_replay::ReplaySerializeError,
    },
    #[error("Failed to serialize metadata '{metadata:?}': {inner}")]
    MetadataSerializeError {
        metadata: libtechmino_replay::GameReplayMetadata,
        inner: serde_json::Error,
    },
    #[error("Failed to serialize input '{input:?}': {inner}")]
    InputSerializeError {
        input: libtechmino_replay::GameInputEvent,
        inner: serde_json::Error,
    },
    #[error("Unexpected end of file")]
    UnexpectedEof,
}

/// The unpacked input event, ready for serialization/deserialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnpackedInputEvent {
    frame: u64,
    r#type: u8,
    key: u8,
}

impl UnpackedInputEvent {
    /// Converts a packed [`GameInputEvent`] into this unpacked input event.
    pub fn from_packed(packed: GameInputEvent) -> Self {
        let InputAction { kind, key } = packed.action();

        let kind: u8 = match kind.into_bool() {
            true => 1,
            false => 0,
        };
        let key = key.into_byte();
        let frame = packed.frame();

        Self {
            frame,
            r#type: kind,
            key,
        }
    }

    /// Attempts to convert this unpacked input event back into the packed [`GameInputEvent`] format.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "This is planned to be used later when we serialize"
        )
    )]
    pub fn try_into_packed(self) -> Option<GameInputEvent> {
        let kind = InputActionKind::try_from(self.r#type).ok()?;
        let key = InputActionKey::try_from_byte(self.key).ok()?;
        let action = InputAction { kind, key };

        GameInputEvent::new(self.frame, action).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNPACKED_AND_SERIALIZED: [(UnpackedInputEvent, &[u8]); 3] = [
        (
            UnpackedInputEvent {
                frame: 626,
                r#type: 1,
                key: 8,
            },
            br#"{"frame":626,"type":1,"key":8}"#.as_slice(),
        ),
        (
            UnpackedInputEvent {
                frame: 0,
                r#type: 0,
                key: 20,
            },
            br#"{"frame":0,"type":0,"key":20}"#.as_slice(),
        ),
        (
            UnpackedInputEvent {
                frame: GameInputEvent::MAX_FRAME,
                r#type: 1,
                key: 20,
            },
            br#"{"frame":9007199254740992,"type":1,"key":20}"#,
        ),
    ];

    const UNPACKED_AND_PACKED: [(UnpackedInputEvent, GameInputEvent); 3] = [
        (
            UnpackedInputEvent {
                frame: 626,
                r#type: 1,
                key: 8,
            },
            new_unpacked_or_panic(626, true, 8),
        ),
        (
            UnpackedInputEvent {
                frame: 0,
                r#type: 0,
                key: 20,
            },
            new_unpacked_or_panic(0, false, 20),
        ),
        (
            UnpackedInputEvent {
                frame: GameInputEvent::MAX_FRAME,
                r#type: 1,
                key: 20,
            },
            new_unpacked_or_panic(GameInputEvent::MAX_FRAME, true, 20),
        ),
    ];

    const fn new_unpacked_or_panic(frame: u64, kind: bool, key: u8) -> GameInputEvent {
        let kind = InputActionKind::from_bool(kind);
        let Ok(key) = InputActionKey::try_from_byte(key) else {
            panic!("invalid key");
        };
        let action = InputAction { kind, key };
        let Ok(event) = GameInputEvent::new(frame, action) else {
            panic!("invalid input; frame overflow?");
        };

        event
    }

    #[test]
    fn serialize_unpacked_inputs() {
        let mut bytes = Vec::with_capacity(UNPACKED_AND_SERIALIZED[0].1.len());

        for (input, expected_json) in UNPACKED_AND_SERIALIZED {
            serde_json::to_writer(&mut bytes, &input).expect("serialization should work");

            assert_eq!(&*bytes, expected_json);

            bytes.clear();
        }
    }

    #[test]
    fn deser_unpacked_inputs() {
        for (expected_unpacked, input_json) in UNPACKED_AND_SERIALIZED {
            let unpacked: UnpackedInputEvent =
                serde_json::from_slice(input_json).expect("deserialization failed");

            assert_eq!(unpacked, expected_unpacked);
        }
    }

    #[test]
    fn conv_unpacked_packed() {
        for (unpacked, exp_packed) in UNPACKED_AND_PACKED {
            let packed = unpacked.try_into_packed().expect("invalid unpacked");

            assert_eq!(packed, exp_packed);
        }
    }

    #[test]
    fn conv_packed_unpacked() {
        for (exp_unpacked, packed) in UNPACKED_AND_PACKED {
            let unpacked = UnpackedInputEvent::from_packed(packed);

            assert_eq!(unpacked, exp_unpacked);
        }
    }
}
