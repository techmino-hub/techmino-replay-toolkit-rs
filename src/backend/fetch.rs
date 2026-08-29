//! Free functions for fetching metadata and input event data from a given
//! replay file.

use std::{
    fs::File,
    io::{self, BufReader, prelude::BufRead},
    path::Path,
};

use libtechmino_replay::{
    consts::METADATA_EVENTDATA_SEPARATOR,
    deserialize::{Decoded, ReplayDecoder, ReplayDecoderPreprocessor},
    errors::ReplayParseError,
    format::ReplayBufferKind,
    replay::{GameInputEvent, GameReplayMetadata},
};

use crate::ParseOrIoError;

/// Reads the metadata from a replay file.
///
/// # Errors
/// Returns an error if something went wrong while extracting the metadata.
pub(in crate::backend) fn fetch_metadata(
    path: &Path,
) -> Result<GameReplayMetadata, ParseOrIoError> {
    let file = File::open(path)?;
    fetch_metadata_inner(BufReader::new(file))
}

/// The inner, I/O-abstracted version of [`fetch_metadata`].
///
/// # Errors
/// Returns an error if something went wrong while extracting the metadata.
fn fetch_metadata_inner<R>(mut reader: R) -> Result<GameReplayMetadata, ParseOrIoError>
where
    R: BufRead,
{
    let first_chunk = reader.fill_buf()?;
    let first_byte = first_chunk.first().ok_or(ReplayParseError::UnexpectedEnd)?;
    let replay_kind = ReplayBufferKind::try_from_first_byte(*first_byte)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{e}")))?;

    let mut processed = Vec::new();
    let mut newly_appended: &[u8] = &[];

    let mut preprocessor = ReplayDecoderPreprocessor::new(replay_kind);

    while !newly_appended.contains(&METADATA_EVENTDATA_SEPARATOR) {
        let input_buf = reader.fill_buf()?;

        if input_buf.is_empty() {
            break;
        }

        let input_len = input_buf.len();

        let old_output_len = processed.len();
        preprocessor
            .preprocess(input_buf, &mut processed)
            .map_err(ReplayParseError::from)?;

        reader.consume(input_len);
        newly_appended = &processed[old_output_len..];
    }

    let separator_pos = processed
        .iter()
        .position(|byte| *byte == METADATA_EVENTDATA_SEPARATOR)
        .ok_or(ReplayParseError::MetadataSeparatorNotFound)?;
    let metadata_bytes = &processed[..separator_pos];

    debug_assert!(
        !metadata_bytes.contains(&METADATA_EVENTDATA_SEPARATOR),
        "Metadata bytes should not have the metadata-eventdata separator"
    );

    let metadata: GameReplayMetadata = serde_json::from_slice(metadata_bytes)
        .map_err(ReplayParseError::MetadataDeserializeError)?;

    Ok(metadata)
}

/// Reads the input event data from a replay file.
///
/// # Errors
/// Returns an error if something went wrong while extracting the input event
/// data.
pub(in crate::backend) fn fetch_input_data(
    path: &Path,
) -> Result<Vec<GameInputEvent>, ParseOrIoError> {
    let file = File::open(path)?;
    fetch_input_data_inner(BufReader::new(file))
}

/// The inner, I/O-abstracted version of [`fetch_input_data`].
///
/// # Errors
/// Returns an error if something went wrong while extracting the input event
/// data.
fn fetch_input_data_inner<R>(mut reader: R) -> Result<Vec<GameInputEvent>, ParseOrIoError>
where
    R: BufRead,
{
    let first_chunk = reader.fill_buf()?;
    let first_byte = first_chunk.first().ok_or(ReplayParseError::UnexpectedEnd)?;
    let replay_kind = ReplayBufferKind::try_from_first_byte(*first_byte)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{e}")))?;

    let mut decoder = ReplayDecoder::new(replay_kind, None);
    let mut events: Vec<GameInputEvent> = Vec::new();

    loop {
        let in_buf = reader.fill_buf()?;

        if in_buf.is_empty() {
            break;
        }

        let in_len = in_buf.len();

        let Decoded {
            metadata: _,
            inputs: new_evs,
            ..
        } = decoder.update(in_buf)?;

        events.extend_from_slice(&new_evs);

        reader.consume(in_len);
    }

    if !decoder.is_finished() {
        return Err(ReplayParseError::UnexpectedEnd.into());
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::test_utils::get_test_cases;

    #[test]
    fn fetchers() {
        let cases = get_test_cases();

        for (name, case) in cases {
            let Some(data) = case.data else {
                eprintln!("Skipping {name} as it does not have a deserialized form");
                continue;
            };

            let Some(path) = case.serialized_path else {
                eprintln!("Skipping {name} as it does not have the path to the deserialized form");
                continue;
            };

            let meta = fetch_metadata(&path).unwrap_or_else(|e| {
                panic!("metadata for {name} should be valid: {e}");
            });
            assert_eq!(data.metadata, meta, "metadata mismatch");

            let events = fetch_input_data(&path).unwrap_or_else(|e| {
                panic!("input event data for {name} should be valid: {e}");
            });
            assert_eq!(data.inputs, events, "input data mismatch");
        }
    }
}
