//! Handles specifically CLI operations and one-off commands.

use crate::cli::{
    clap::{CliOperation, CliReplayFormat},
    types::CliOpError,
};
use libtechmino_replay::config::ReplayBufferKind;

mod base64ify;
mod binaryify;
mod create;
mod extract;

pub fn handle_cli_op(operation: &CliOperation) -> Result<(), CliOpError> {
    match operation {
        CliOperation::Extract(args) => extract::extract(args),
        CliOperation::Create(args) => create::create(args),
        CliOperation::Base64ify(args) => base64ify::base64ify(args),
        CliOperation::Binaryify(args) => binaryify::binaryify(args),
    }
}

/// Infers the replay kind from the first byte of the encoded replay.
fn infer_replay_kind(
    fmt_override: Option<CliReplayFormat>,
    first_chunk: &[u8],
) -> Result<ReplayBufferKind, CliOpError> {
    if let Some(format) = fmt_override {
        return Ok(format.into());
    }

    let first_byte = first_chunk.first().copied().ok_or(CliOpError::InputEmpty)?;

    ReplayBufferKind::try_from_first_byte(first_byte)
        .map_err(|_| CliOpError::ReplayKindInferFailed { first_byte })
}
