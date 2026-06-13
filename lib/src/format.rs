//! Module for representing the three different ways to have inputs/outputs in this library.

/// Represents the different kinds of ways that Techmino replays could be represented.
///
/// The most common ones are [`Base64`][Self::Base64] for copy-pasteable text from the
/// "Replays" menu and the [`Compressed`][Self::Compressed] from the `.rep` files in the
/// save directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReplayBufferKind {
    /// Base64-encoding of zlib-compressed bytes.
    ///
    /// This is the form of replay data you get when you copy
    /// a replay in the game's "Replays" menu.
    Base64,
    /// Zlib-compressed bytes.
    ///
    /// This is the form of replay data the game stores in
    /// `<game_save_dir>/replay/*.rep`.
    Compressed,
    /// Uncompressed bytes.
    ///
    /// This form is not seen anywhere in the game, but useful
    /// if you have already decompressed a `.rep` file in
    /// memory.
    Uncompressed,
}
