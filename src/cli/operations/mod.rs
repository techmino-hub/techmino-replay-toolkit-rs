//! Handles specifically CLI operations and one-off commands.

use crate::cli::{
    clap::{CliOperation, CliReplayFormat},
    types::CliOpError,
};
use libtechmino_replay::ReplayBufferKind;

mod base64ify;
mod binaryify;
mod create;
mod extract;
mod shrink;

pub fn handle_cli_op(operation: &CliOperation) -> Result<(), CliOpError> {
    match operation {
        CliOperation::Extract(args) => extract::extract(args),
        CliOperation::Create(args) => create::create(args),
        CliOperation::Base64ify(args) => base64ify::base64ify(args),
        CliOperation::Binaryify(args) => binaryify::binaryify(args),
        CliOperation::Shrink(args) => shrink::shrink(args),
    }
}

/// Infers the replay kind from the first byte of the encoded replay.
fn infer_replay_kind(
    fmt_override: Option<CliReplayFormat>,
    first_chunk: &[u8],
) -> Result<ReplayBufferKind, CliOpError> {
    /// Zlib always begins with 0x78 (`x`): https://en.wikipedia.org/wiki/List_of_file_signatures
    const ZLIB_HEADER_FIRST_BYTE: u8 = b'x';
    /// 0x7800 until 0x78FF always starts with an `e` in base64
    const BASE64_ZLIB_FIRST_BYTE: u8 = b'e';
    /// Raw uncompressed game data begins with a JSON object, which begins with a `{`
    const UNCOMPRESSED_FIRST_BYTE: u8 = b'{';

    if let Some(format) = fmt_override {
        return Ok(format.into());
    }

    let first_byte = first_chunk.first().copied().ok_or(CliOpError::InputEmpty)?;

    match first_byte {
        ZLIB_HEADER_FIRST_BYTE => Ok(ReplayBufferKind::Compressed),
        BASE64_ZLIB_FIRST_BYTE => Ok(ReplayBufferKind::Base64),
        UNCOMPRESSED_FIRST_BYTE => Ok(ReplayBufferKind::Uncompressed),
        _ => Err(CliOpError::ReplayKindInferFailed { first_byte }),
    }
}
