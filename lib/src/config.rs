//! Configurations for the replay encoder/decoder.

use alloc::{string::String, vec::Vec};
#[cfg(feature = "arbitrary")]
use arbitrary::Arbitrary;
use semver::Version;

use crate::{
    GameReplayMetadata, ReplaySerializeError,
    consts::{BASE64_ZLIB_FIRST_BYTE, UNCOMPRESSED_FIRST_BYTE, ZLIB_HEADER_FIRST_BYTE},
    errors::UnknownReplayKind,
    serialize::ReplayEncoder,
};

/// Represents the different kinds of ways that Techmino replays could be represented.
///
/// The most common ones are [`Base64`][Self::Base64] for copy-pasteable text from the
/// "Replays" menu and the [`Compressed`][Self::Compressed] from the `.rep` files in the
/// save directory.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
    ///
    /// This format will not be directly playable using the unmodified game.
    Uncompressed,
}

impl ReplayBufferKind {
    /// Returns whether or not this replay kind involves non-ASCII characters.
    ///
    /// # Disambiguation
    /// Note that this does NOT just refer to the binary `.rep` files; this
    /// functions not only detect the compressed binary `.rep` files, but also
    /// any decompressed forms of replays.
    ///
    /// # General
    /// If you want to narrow down to ONLY `.rep` file formats (compressed
    /// binary), use the [`Self::is_binary_compressed`] method.
    #[must_use]
    pub const fn is_binary(self) -> bool {
        match self {
            Self::Base64 => false,
            Self::Compressed | Self::Uncompressed => true,
        }
    }

    /// Returns whether or not this replay kind is *specifically* the
    /// compressed binary form.
    ///
    /// # Specialized
    /// If you want to generalize to all kinds that use compression in any way,
    /// use the [`Self::is_compressed`] method.
    #[must_use]
    pub const fn is_binary_compressed(self) -> bool {
        match self {
            Self::Compressed => true,
            Self::Base64 | Self::Uncompressed => false,
        }
    }

    /// Returns whether or not this replay kind is *specifically* the
    /// uncompressed binary form.
    ///
    /// # Uncommon
    /// Uncompressed binary replays aren't created by the game and may be the
    /// result of external tooling. If you mean to detect `.rep` files, they
    /// use the **compressed** form instead, in which case you should
    /// consider using [`Self::is_binary_compressed`] instead.
    #[must_use]
    pub const fn is_binary_uncompressed(self) -> bool {
        match self {
            Self::Base64 | Self::Compressed => false,
            Self::Uncompressed => true,
        }
    }

    /// Returns whether or not this replay kind is of the compressed base64
    /// form.
    #[must_use]
    pub const fn is_base64(self) -> bool {
        match self {
            Self::Base64 => true,
            Self::Compressed | Self::Uncompressed => false,
        }
    }

    /// Returns whether or not this replay kind utilizes compression to shrink
    /// the final size.
    ///
    /// # General
    /// This function covers both compressed binary and compressed base64. If
    /// you specifically want to detect either of these, consider using
    /// [`Self::is_binary_compressed`] or [`Self::is_base64`] instead.
    #[must_use]
    pub const fn is_compressed(self) -> bool {
        match self {
            Self::Base64 | Self::Compressed => true,
            Self::Uncompressed => false,
        }
    }

    /// Tries to infer the replay kind from the first byte of the replay stream.
    ///
    /// # Heuristics
    /// This function is heuristics-based and may not be 100% accurate.
    ///
    /// # Errors
    /// Returns if the given byte does not match any known first byte patterns.
    pub const fn try_from_first_byte(byte: u8) -> Result<Self, UnknownReplayKind> {
        match byte {
            UNCOMPRESSED_FIRST_BYTE => Ok(Self::Uncompressed),
            ZLIB_HEADER_FIRST_BYTE => Ok(Self::Compressed),
            BASE64_ZLIB_FIRST_BYTE => Ok(Self::Base64),
            _ => Err(UnknownReplayKind { first_byte: byte }),
        }
    }
}

/// Determines how to parse the inputs of the replay.
///
/// Replays made before version 0.17.22 of the game (i.e., 0.17.21 and before it)
/// use relative timing for its inputs.\
/// However, starting from version 0.17.22 of the game, absolute timing is used.
#[cfg_attr(feature = "arbitrary", derive(Arbitrary))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputParseMode {
    /// Relative timing.
    ///
    /// Replays made before version 0.17.22 of the game (i.e., 0.17.21 and before it)
    /// use relative timing for its inputs. That is, the time in each key-time
    /// pair are relative to the frame of the previous input.
    ///
    /// For example, if you press two keys at the exact same frame, the first input
    /// has a stored time of the number of frames since the previous input,
    /// while the second input has a time of 0.
    Relative,
    /// Absolute timing.
    ///
    /// Replays made after version 0.17.21 of the game (i.e., 0.17.22 and onwards)
    /// use absolute timing for its inputs. That is, the time in each key-time
    /// pair are relative to the beginning of the replay (i.e., frame zero).
    ///
    /// For example, if you press two keys at the exact same frame, the first input
    /// has a time of the current frame number, as well as the second input.
    Absolute,
}

impl InputParseMode {
    /// The first version where absolute timing is used.
    pub const ABSOLUTE_TIMING_START: Version = Version::new(0, 17, 22);

    /// Tries to infer the input parse mode based on the game version.
    ///
    /// If parsing the version fails, it will return `None`.
    #[must_use]
    pub fn try_infer_from_version(version: &str) -> Option<InputParseMode> {
        let lower = version.to_ascii_lowercase();
        let lower = lower
            .trim_start_matches('v')
            .trim_start_matches("alpha")
            .trim_start();

        if lower.contains("wtf") {
            // Matches Techmino WTF mod from April 2024
            // https://github.com/MelloBoo44/Techmino-WTF
            return Some(InputParseMode::Relative);
        }

        if lower.trim_start().starts_with("unofficial expansion") {
            // Matches Techmino Unofficial Expansion mod from August 2023
            // https://github.com/Another-Soul/Techmino-Unofficial-Expansion
            return Some(InputParseMode::Relative);
        }

        // Snapshots use @ as version@commit delimiter
        let lower = match lower.find('@') {
            Some(idx) => &lower[..idx],
            None => lower,
        };

        // Electra's mods have multiple elements to them
        let lower = lower.split(' ').next().unwrap_or_default();

        let filtered_version: String = lower
            .chars()
            .filter(|c| c.is_numeric() || *c == '.')
            .collect();

        let version = Version::parse(&filtered_version);

        if let Ok(v) = version {
            if v < Self::ABSOLUTE_TIMING_START {
                return Some(InputParseMode::Relative);
            }
            return Some(InputParseMode::Absolute);
        }

        None
    }

    /// Tries to infer the input parse mode based on the input slice.
    ///
    /// Returns [`None`] if the input parse mode could not be inferred.
    #[must_use]
    #[deprecated = "this doesn't really fit in with the rest of the codebase that uses VlqReader instead"]
    pub fn try_infer_from_input_data(input_slice: &[u64]) -> Option<InputParseMode> {
        // Absolute mode: expects increasing frame times
        let mut prev_time = 0;
        for &time in input_slice.iter().step_by(2) {
            if time < prev_time {
                // Definitely not absolute!
                return Some(InputParseMode::Relative);
            }

            prev_time = time;
        }

        // It's not really possible to "disprove" relative mode, so we're still unsure
        None
    }
}

/// Configures a [`ReplayEncoder`].
///
/// # Defaults
/// By default, the encoder is configured to make a replay in a compatible
/// format as `.rep` files.
///
/// # Example
/// ```
/// use libtechmino_replay::{
///     config::{EncoderConfig, ReplayBufferKind},
///     replay::GameReplayMetadata,
///     serialize::ReplayEncoder,
/// };
///
/// let mut metadata: GameReplayMetadata = GameReplayMetadata::new();
/// metadata.set_version("V0.17.21");
///
/// let (mut encoder, mut buffer): (ReplayEncoder, Vec<u8>) = EncoderConfig::DEFAULT
///     .kind(ReplayBufferKind::Base64)
///     .build(&metadata)
///     .unwrap();
/// ```
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug)]
pub struct EncoderConfig {
    /// The target format of the replay.
    pub(crate) replay_kind: ReplayBufferKind,
    /// The level of compression to apply to the replay, if the replay kind
    /// is compressed.
    pub(crate) compression_level: u8,
    /// The input mode to use, if overridden.
    pub(crate) input_mode_override: Option<InputParseMode>,
}

impl EncoderConfig {
    /// The default encoder config.
    ///
    /// By default, the encoder is configured to make a replay in a format
    /// compatible with the `.rep` files made by the game.
    pub const DEFAULT: Self = Self {
        replay_kind: ReplayBufferKind::Compressed,
        compression_level: 1,
        input_mode_override: None,
    };

    /// Creates a new encoder config for a given replay format.
    #[must_use]
    pub const fn new(replay_kind: ReplayBufferKind) -> Self {
        Self::DEFAULT.kind(replay_kind)
    }

    /// Sets the desired target format of the replay.
    ///
    /// Defaults to [`ReplayBufferKind::Compressed`].
    ///
    /// # Example
    /// ```
    /// # use libtechmino_replay::config::{EncoderConfig, ReplayBufferKind};
    /// let conf = EncoderConfig::DEFAULT
    ///     .kind(ReplayBufferKind::Compressed);
    /// ```
    #[must_use = "this function returns the modified self"]
    pub const fn kind(mut self, replay_kind: ReplayBufferKind) -> Self {
        self.replay_kind = replay_kind;
        self
    }

    /// Gets the current desired target format of the replay.
    ///
    /// # Example
    /// ```
    /// # use libtechmino_replay::config::{EncoderConfig, ReplayBufferKind};
    /// let conf = EncoderConfig::DEFAULT;
    ///
    /// assert_eq!(conf.get_kind(), ReplayBufferKind::Compressed);
    /// ```
    #[must_use]
    pub const fn get_kind(&self) -> ReplayBufferKind {
        self.replay_kind
    }

    /// The DEFLATE compression level to apply to the replay.
    ///
    /// The default of 1 is a decent trade-off between time and size for
    /// Techmino TASes and replays.
    ///
    /// This has no effect on the uncompressed replay data format.\
    /// For more information, see [`ReplayBufferKind::is_compressed()`].
    ///
    /// Note that the given level will be processed by miniz-oxide. It decides
    /// what do to with the number given. Currently it clamps it to a maximum
    /// of 10.
    ///
    /// For more information, see [`miniz_oxide::deflate::CompressionLevel`].
    ///
    /// # Example
    /// ```
    /// # use libtechmino_replay::config::EncoderConfig;
    /// let conf = EncoderConfig::DEFAULT
    ///     .compression_level(1);
    /// ```
    #[must_use = "this function returns the modified self"]
    pub const fn compression_level(mut self, level: u8) -> Self {
        self.compression_level = level;
        self
    }

    /// Gets the stored desired compression level for the replay.
    ///
    /// For more information, see [the setter][Self::compression_level].
    #[must_use]
    pub const fn get_compression_level(&self) -> u8 {
        self.compression_level
    }

    /// Overrides the input parse mode.
    ///
    /// Default: None (no override).
    ///
    /// # Input Parse Mode
    /// This function takes in an input parse mode override. This is often not required, but can be useful
    /// if you're targeting a mod and this library fails to infer the input parse mode from the version.
    ///
    /// Passing in the wrong input parse mode will result in nonsensical inputs, though, so it's usually
    /// best to give a `None` and let the library infer the input parse mode from the metadata's version
    /// string.
    ///
    /// For more information, see [`InputParseMode`].
    #[must_use = "this function returns the modified self"]
    pub const fn input_mode(mut self, mode: Option<InputParseMode>) -> Self {
        self.input_mode_override = mode;
        self
    }

    /// Gets the current input parse mode override, if any.
    ///
    /// For more information, see [the setter][Self::input_mode].
    #[must_use]
    pub const fn get_input_mode(&self) -> Option<InputParseMode> {
        self.input_mode_override
    }

    /// Initializes the [`ReplayEncoder`] based on the current config.
    ///
    /// # Metadata
    /// This function takes in a metadata input, where the version is checked in
    /// order to determine the input parse mode.
    ///
    /// If you'd like to skip this check, use [`Self::input_mode`] to give your
    /// own input mode instead.
    ///
    /// # Errors
    /// For more information on errors, see [`ReplayEncoder::with_config()`].
    ///
    /// # Example
    /// ```
    /// # use libtechmino_replay::{config::EncoderConfig, serialize::ReplayEncoder, replay::GameReplayMetadata};
    ///
    /// let mut metadata = GameReplayMetadata::new();
    /// metadata.set_version("V0.17.22");
    ///
    /// let (mut encoder, mut buffer): (ReplayEncoder, Vec<u8>) = EncoderConfig::DEFAULT
    ///     .build(&metadata)
    ///     .unwrap();
    ///
    /// // Use the encoder...
    /// ```
    #[must_use = "encode a replay with the encoder"]
    pub fn build(
        &self,
        metadata: &GameReplayMetadata,
    ) -> Result<(ReplayEncoder, Vec<u8>), ReplaySerializeError> {
        ReplayEncoder::with_config(metadata, self)
    }
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use fastrand::Rng;
    use strum::IntoEnumIterator;

    use super::*;
    use crate::replay::{
        GameInputEvent,
        action::{InputAction, InputActionKey, InputActionKind},
    };

    #[test]
    fn test_inferred_mode() {
        use InputParseMode::*;
        let cases = [
            ("Techmino is fun!", None),
            ("Alpha v0.15.1", Some(Relative)),
            ("V0.16.2", Some(Relative)),
            ("0.17.22", Some(Absolute)),
            ("v0.17.6@26fc", Some(Relative)),
            ("v 1.2.3", Some(Absolute)),
            // https://github.com/MelloBoo44/Techmino-WTF/blob/main/version.lua
            ("WTF", Some(Relative)),
            // https://github.com/Another-Soul/Techmino-Unofficial-Expansion/blob/main/version.lua
            ("Unofficial Expansion v0.2.1", Some(Relative)),
            // https://github.com/electraminer/Techmino/blob/king_of_stackers/version.lua
            (
                "V0.17.22 IRSv1.1 PASSTHROUGHFIXv1.0 KOSv1.2beta TE:Cv1.0",
                Some(Absolute),
            ),
            // https://github.com/electraminer/Techmino/blob/irs/version.lua
            ("V0.17.22 + IRSv1.1.1", Some(Absolute)),
            // https://github.com/electraminer/Techmino/blob/king_of_cheesers/version.lua
            (
                "V0.17.22 IRSv1.1 PASSTHROUGHFIXv1.0 KOCv0.1beta TE:Cv1.0",
                Some(Absolute),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(InputParseMode::try_infer_from_version(input), expected);
        }
    }

    #[cfg(feature = "strum")]
    #[test]
    fn test_event_roundtrip() {
        const ROUNDS: usize = 10_000_000;

        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for i in 0..ROUNDS {
            let kind: InputActionKind = rng.bool().into();
            let key = rng.choice(InputActionKey::iter()).unwrap();
            let action = InputAction { kind, key };
            let frame = rng.u64(0..=GameInputEvent::MAX_FRAME);

            let Ok(event) = GameInputEvent::new(frame, action) else {
                panic!(
                    "Failed to create GameInputEvent from args:
                    Kind: {kind:?} = {kind_discriminant:?}
                    Key: {key:?} = {key_discriminant:?}
                    Frame: {frame} = {frame:x}",
                    kind_discriminant = core::mem::discriminant(&kind),
                    key_discriminant = core::mem::discriminant(&key),
                );
            };

            let (rt_kind, rt_key, rt_frame) = (event.kind(), event.key(), event.frame());

            assert_eq!(kind, rt_kind);
            assert_eq!(key, rt_key);
            assert_eq!(frame, rt_frame);

            if i % 1_000_000 == 0 {
                eprintln!("{i} of {ROUNDS}");
            }
        }
    }
}
