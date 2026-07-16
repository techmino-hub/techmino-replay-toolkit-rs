use crate::cli::clap::{CliInputMode, CliReplayFormat, ExtractMode, RetryArguments};
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
pub(crate) enum CliOpError {
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
}
