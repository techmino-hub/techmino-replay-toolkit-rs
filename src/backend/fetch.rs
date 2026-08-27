//! Free functions for fetching metadata and input event data from a given
//! replay file.

use std::path::Path;

use libtechmino_replay::replay::{GameInputEvent, GameReplayMetadata};

use crate::ParseOrIoError;

/// Reads the metadata from a replay file.
///
/// # Errors
/// Returns an error if something went wrong while
pub(in crate::backend) fn fetch_metadata(
    path: &Path,
) -> Result<GameReplayMetadata, ParseOrIoError> {
    todo!();
}

pub(in crate::backend) fn fetch_input_data(
    path: &Path,
) -> Result<Vec<GameInputEvent>, ParseOrIoError> {
    todo!();
}
