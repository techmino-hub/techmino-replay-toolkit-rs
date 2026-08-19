//! Configurations for the replay encoder/decoder.

use alloc::string::String;
#[cfg(feature = "arbitrary")]
use arbitrary::Arbitrary;
use semver::Version;

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
