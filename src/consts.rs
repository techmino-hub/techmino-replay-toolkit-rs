//! Constants for techmino-replay-toolkit.

/////////////// REPLAY DATA JSON ///////////////

/// The "metadata" keyword for the replay data JSON.
///
/// Useful to form/parse e.g. `{"metadata":...}`
pub(crate) static KEYWORD_METADATA: &str = "metadata";

/// The "inputs" keyword for the replay data JSON.
///
/// Useful to form/parse e.g. `{"inputs":...}`
pub(crate) static KEYWORD_INPUTS: &str = "inputs";

/// The "frame" keyword for the replay data JSON's input data.
///
/// Useful to form/parse e.g. `{"frame":...}`
pub(crate) static KEYWORD_FRAME: &str = "frame";

/// The "type" keyword for the replay data JSON's input data.
///
/// Useful to form/parse e.g. `{"type":...}`
pub(crate) static KEYWORD_TYPE: &str = "type";

/// The "key" keyword for the replay data JSON's input data.
///
/// Useful to form/parse e.g. `{"key":...}`
pub(crate) static KEYWORD_KEY: &str = "key";

/// The metadata key for the "created with techmino-replay-toolkit" hidden marker.
pub(crate) static TRT_CREATION_MARKER_KEY: &str = "__meta__created_with_techmino_replay_toolkit";

/// The (current) metadata value for the "created with techmino-replay-toolkit" hidden marker.
pub(crate) static TRT_CREATION_MARKER_VALUE: &str = env!("MARKER_VALUE");
