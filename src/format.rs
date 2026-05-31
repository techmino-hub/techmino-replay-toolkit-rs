//! Module for representing the three different ways to have inputs/outputs in this library.

/// Represents the different kinds of ways that Techmion replays could be represented.
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
