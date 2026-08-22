//! Compatibility layer for former `format` module.

/// Deprecated compatibility layer.
///
/// ```
/// # // If this test fails, update the deprecation notice too
/// use libtechmino_replay::config::ReplayBufferKind;
/// ```
#[deprecated(
    since = "0.2.0",
    note = "relocated to `libtechmino_replay::config::ReplayBufferKind`"
)]
pub use crate::config::ReplayBufferKind;

/// Deprecated compatibility layer.
///
/// ```
/// # // If this test fails, update the deprecation notice too
/// use libtechmino_replay::replay::SerializedReplay;
/// ```
#[deprecated(
    since = "0.2.0",
    note = "relocated to `libtechmino_replay::replay::SerializedReplay`"
)]
pub use crate::replay::SerializedReplay;
