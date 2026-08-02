//! Heuristics used by the TUI.
//!
//! The implementation details shall be regarded as unstable, since they are merely
//! heuristics, after all.
// TODO(doc, post-GUI): Does the GUI use this? If so update docs

use std::{
    fs::File,
    io::{
        SeekFrom,
        prelude::{Read, Seek},
    },
};

use crate::consts;
use base64::engine::{Engine as _, general_purpose::STANDARD as B64};
use libtechmino_replay::ReplayBufferKind;
use miniz_oxide::{
    MZError, MZFlush, MZStatus,
    inflate::{
        TINFLStatus,
        stream::{InflateState, inflate},
    },
};

/// Detects whether or not a replay file is likely a valid replay file.
///
/// # Implementation Details
/// Right now, it reads from the beginning of the file, until one chunk after
/// the end of the metadata.
///
/// It does some checks:
/// - Checks that I/O functions properly
/// - Checks that the first byte matches one of the known magic patterns
/// - Checks that the decode and decompress works fine (if it needs to be)
/// - Checks that the metadata section is valid ASCII
/// - Checks that the input event data is not ASCII
///
/// Note that these heuristics' implementation details may change at any time.
pub(crate) fn likely_valid_replay_file(file: &mut File) -> bool {
    /// How much input data to process before bailing out.
    const INPUT_SIZE_LIMIT: usize = 1 << 20;

    let Ok(0) = file.seek(SeekFrom::Start(0)) else {
        return false;
    };

    let input_size = file
        .metadata()
        .map(|m| usize::try_from(m.len()).unwrap_or(INPUT_SIZE_LIMIT))
        .unwrap_or(INPUT_SIZE_LIMIT);

    let mut input = vec![0u8; input_size];

    let Ok(()) = file.read_exact(&mut input) else {
        return false;
    };

    let kind = match input.first().copied() {
        Some(consts::ZLIB_HEADER_FIRST_BYTE) => ReplayBufferKind::Compressed,
        Some(consts::BASE64_ZLIB_FIRST_BYTE) => ReplayBufferKind::Base64,
        Some(consts::UNCOMPRESSED_FIRST_BYTE) => ReplayBufferKind::Uncompressed,
        _unrecognized => return false,
    };

    // Decode from base64
    let decoded = match kind {
        ReplayBufferKind::Uncompressed | ReplayBufferKind::Compressed => input,
        ReplayBufferKind::Base64 => {
            // For clean decode, we need data length divisible by four
            let data_len = if input.len().is_multiple_of(4) {
                input.len()
            } else {
                input.len().next_multiple_of(4).saturating_sub(4)
            };
            let Ok(vec) = B64.decode(&input[..data_len]) else {
                return false;
            };
            vec
        }
    };

    let decompressed = match kind {
        ReplayBufferKind::Uncompressed => decoded,
        ReplayBufferKind::Compressed | ReplayBufferKind::Base64 => {
            let Some(d) = decompress_partial(&decoded) else {
                return false;
            };
            d
        }
    };

    let mut parts = decompressed.as_slice().splitn(2, |byte| *byte == b'\n');
    let Some(metadata_bytes) = parts.next() else {
        return false;
    };
    let Some(inputdata_bytes) = parts.next() else {
        return false;
    };

    if !metadata_bytes.is_ascii() {
        return false;
    }

    if inputdata_bytes.is_ascii() {
        return false;
    }

    true
}

/// Attempts to decompress zlib-compressed partial data.
fn decompress_partial(mut compressed: &[u8]) -> Option<Vec<u8>> {
    /// The max size (in bytes) of the decompression heap buffer before bailing out.
    const DECOMPRESS_SIZE_LIMIT: usize = 1 << 28;
    /// The size of the stack-allocated scratch buffer in bytes.
    const SCRATCH_BUFFER_SIZE: usize = 4096;

    let mut state = InflateState::new(miniz_oxide::DataFormat::Zlib);
    let mut scratch = [0u8; SCRATCH_BUFFER_SIZE];
    let mut decompressed = Vec::with_capacity(2 * compressed.len());

    loop {
        let res = inflate(&mut state, compressed, &mut scratch, MZFlush::None);

        decompressed.extend_from_slice(&scratch[..res.bytes_written]);
        compressed = &compressed[res.bytes_consumed..];
        if decompressed.len() > DECOMPRESS_SIZE_LIMIT {
            break;
        }

        match res.status {
            Ok(MZStatus::StreamEnd) => break,
            Ok(_) => continue,
            Err(MZError::Buf) if state.last_status() == TINFLStatus::NeedsMoreInput => {
                break;
            }
            Err(_) => return None,
        }
    }

    Some(decompressed)
}
