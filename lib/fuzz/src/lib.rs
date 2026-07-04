use libfuzzer_sys::arbitrary::{unstructured::Unstructured, Arbitrary};
use libtechmino_replay::{
    deserialize::ReplayDecoder,
    serialize::ReplayEncoder,
    GameInputEvent, GameReplayData, GameReplayMetadata, InputParseMode,
    ReplayBufferKind::{self},
    ReplayParseError,
};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::VecDeque;

pub const MAX_METADATA_DEPTH: usize = 120;

/// Struct for fuzzing streaming encoding.
#[derive(Debug)]
pub struct EncodeStream {
    /// The entire game replay data to encode
    pub total_game_data: GameReplayData,
    /// The list of indices to encode in each pass.
    ///
    /// Each `.update()` pass will insert a certain slice of the
    /// total game replay data's input list.
    ///
    /// If we are in pass `p` where `p < indices.len()`, then we will call
    /// `.update()` using `inputs[indices[p - 1]..indices[p]]`,
    /// or if p == 0, then `inputs[..indices[p]]`
    pub indices: Vec<usize>,
    pub rep_kind: ReplayBufferKind,
    pub compression_level: u8,
    pub input_mode_override: Option<InputParseMode>,
}

impl EncodeStream {
    fn metadata(&self) -> &GameReplayMetadata {
        &self.total_game_data.metadata
    }

    fn inputs(&self) -> &[GameInputEvent] {
        &self.total_game_data.inputs
    }

    pub fn test(&self) -> Result<Vec<u8>, libtechmino_replay::ReplaySerializeError> {
        let mut encoder = ReplayEncoder::new(self.rep_kind, self.compression_level);
        let mut serialized = encoder.feed_metadata(self.metadata(), self.input_mode_override)?;

        for pass in 0..self.indices.len() {
            let lower_bound = pass.checked_sub(1).map(|p| self.indices[p]).unwrap_or(0);
            let upper_bound = self.indices[pass];
            let input_slice = &self.inputs()[lower_bound..upper_bound];

            encoder.feed_input_data(input_slice, &mut serialized)?;
        }

        encoder.finish(&mut serialized)?;

        drop(encoder);

        let decoded =
            GameReplayData::parse_replay(&serialized, self.rep_kind, self.input_mode_override)
                .expect("decode failed");

        assert_eq!(self.total_game_data, decoded);

        Ok(serialized)
    }
}

impl<'a> Arbitrary<'a> for EncodeStream {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let rep_kind = ReplayBufferKind::arbitrary(u)?;
        let compression_level = u.int_in_range(0u8..=10u8)?;
        let input_mode_override = u.arbitrary()?;

        let total_game_data = GameReplayData::arbitrary(u)?;

        // Would get decode failure otherwise (recursion depth exceeded)
        if get_metadata_max_depth(&total_game_data.metadata) > MAX_METADATA_DEPTH {
            return Err(arbitrary::Error::IncorrectFormat);
        }

        let input_count = total_game_data.inputs.len();

        let mut indices = Vec::new();
        let mut prev_index = 0;

        while prev_index < input_count {
            if u.is_empty() {
                return Err(arbitrary::Error::NotEnoughData);
            }

            let next_index = u.int_in_range(prev_index..=input_count)?;
            indices.push(next_index);
            prev_index = next_index;
        }

        Ok(Self {
            total_game_data,
            indices,
            rep_kind,
            compression_level,
            input_mode_override,
        })
    }
}

/// `impl Arbitrary` struct for representing how a decode stream shall be fuzzed.
#[derive(Debug)]
pub struct DecodeStream {
    /// The entire source game replay data.
    pub source_data: GameReplayData,
    /// The encoded game replay data.
    pub replay_bytes: Box<[u8]>,
    /// The list of indices to encode in each pass.
    ///
    /// Each `.update()` pass will insert a certain slice of the
    /// total encoded game replay data.
    ///
    /// If we are in pass `p` where `p < indices.len()`, then we will call
    /// `.update()` using `replay_bytes[indices[p - 1]..indices[p]]`,
    /// or if p == 0, then `replay_bytes[..indices[p]]`
    pub indices: Vec<usize>,
    /// How the encoded bytes shall be decoded.
    pub format: ReplayBufferKind,
    /// What mode was used to encode the replay.
    _encode_mode: InputParseMode,
    /// An input mode override to apply.
    pub decode_mode_override: Option<InputParseMode>,
    /// What to expect from decoding.
    pub expectation: DecodeStreamExpectation,
}

/// What to expect as the fuzz results.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecodeStreamExpectation {
    /// The decode will fail to succeed.
    DecodeFail,
    /// The decode may not succeed.
    ///
    /// No assertions.
    NoExpectation,
    /// The decode is expected to succeed, but with no expectations about
    /// roundtrips.
    Decodes,
    /// The decode is expected to succeed and roundtrip properly.
    Roundtrips,
}

impl DecodeStreamExpectation {
    /// What to expect only from looking at parse modes and metadata.
    pub fn infer(
        encode_mode: InputParseMode,
        decode_override: Option<InputParseMode>,
        metadata: &GameReplayMetadata,
    ) -> Self {
        let inferred_decode_mode = if let Some(mode) = decode_override {
            mode
        } else {
            let Some(Ok(version)) = metadata.get_version() else {
                return DecodeStreamExpectation::DecodeFail;
            };
            let Some(mode) = InputParseMode::try_infer_from_version(version) else {
                return DecodeStreamExpectation::DecodeFail;
            };
            mode
        };

        let mismatch_expectation = match (encode_mode, inferred_decode_mode) {
            (InputParseMode::Absolute, InputParseMode::Relative)
            | (InputParseMode::Relative, InputParseMode::Absolute) => {
                DecodeStreamExpectation::NoExpectation
            }
            (InputParseMode::Absolute, InputParseMode::Absolute)
            | (InputParseMode::Relative, InputParseMode::Relative) => {
                DecodeStreamExpectation::Roundtrips
            }
        };

        let recursion_expectation = if get_metadata_max_depth(metadata) > MAX_METADATA_DEPTH {
            DecodeStreamExpectation::NoExpectation
        } else {
            DecodeStreamExpectation::Roundtrips
        };

        recursion_expectation.min(mismatch_expectation)
    }

    pub fn minimize(&mut self, rhs: Self) {
        *self = (*self).min(rhs);
    }
}

impl DecodeStream {
    pub fn test(&self) {
        let result = self.try_decode();

        if !self.meets_expectations(&result) {
            dbg!(self);
            dbg!(&result);

            panic!("decode failed to meet expectations");
        }
    }

    fn try_decode(&self) -> Result<GameReplayData, ReplayParseError> {
        let mut decoder = ReplayDecoder::new(self.format, self.decode_mode_override);
        let mut metadata: Option<GameReplayMetadata> = None;
        let mut inputs: Vec<GameInputEvent> = Vec::with_capacity(self.source_data.inputs.len());

        for pass in 0..self.indices.len() {
            let lower_bound = pass.checked_sub(1).map(|p| self.indices[p]).unwrap_or(0);
            let upper_bound = self.indices[pass];
            let input_slice = &self.replay_bytes[lower_bound..upper_bound];

            let res = decoder.update(input_slice)?;

            if let Some(meta_output) = res.metadata {
                metadata = Some(*meta_output);
            }

            inputs.extend_from_slice(&res.inputs);
        }

        if !decoder.is_finished() {
            return Err(ReplayParseError::UnexpectedEnd);
        }

        let data = GameReplayData {
            metadata: metadata.ok_or(ReplayParseError::MetadataSeparatorNotFound)?,
            inputs,
        };

        Ok(data)
    }

    fn meets_expectations(&self, result: &Result<GameReplayData, ReplayParseError>) -> bool {
        match self.expectation {
            DecodeStreamExpectation::DecodeFail => result.is_err(),
            DecodeStreamExpectation::NoExpectation => true,
            DecodeStreamExpectation::Decodes => result.is_ok(),
            DecodeStreamExpectation::Roundtrips => {
                result.is_ok() && *result.as_ref().unwrap() == self.source_data
            }
        }
    }
}

impl<'a> Arbitrary<'a> for DecodeStream {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut expectation = DecodeStreamExpectation::Roundtrips;
        let source_data = GameReplayData::arbitrary(u)?;

        let format = ReplayBufferKind::arbitrary(u)?;

        let compression_level = if format == ReplayBufferKind::Uncompressed {
            0
        } else {
            u.int_in_range(0u8..=10u8)?
        };

        let encode_mode: InputParseMode = u.arbitrary()?;

        let replay_bytes: Box<[u8]> = source_data
            .serialize(format, Some(encode_mode), compression_level)
            .map_err(|_| arbitrary::Error::IncorrectFormat)?
            .into();

        let decode_override: Option<InputParseMode> = u.arbitrary()?;

        expectation.minimize(DecodeStreamExpectation::infer(
            encode_mode,
            decode_override,
            &source_data.metadata,
        ));

        let mut indices: Vec<usize> = Vec::with_capacity(1);
        let mut prev_idx = 0usize;

        while prev_idx < replay_bytes.len() {
            if u.is_empty() {
                return Err(arbitrary::Error::NotEnoughData);
            }

            let new_idx = u.int_in_range(prev_idx..=replay_bytes.len())?;
            indices.push(new_idx);
            prev_idx = new_idx;
        }

        Ok(Self {
            source_data,
            replay_bytes,
            format,
            indices,
            _encode_mode: encode_mode,
            decode_mode_override: decode_override,
            expectation,
        })
    }
}

enum RecursibleJson<'a> {
    Object(&'a JsonMap<String, JsonValue>),
    Array(&'a [JsonValue]),
}

enum RecursibleJsonIter<'a> {
    Object(serde_json::map::Values<'a>),
    Array(core::slice::Iter<'a, JsonValue>),
}

impl<'a> IntoIterator for RecursibleJson<'a> {
    type IntoIter = RecursibleJsonIter<'a>;
    type Item = &'a JsonValue;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Object(m) => RecursibleJsonIter::Object(m.values()),
            Self::Array(arr) => RecursibleJsonIter::Array(arr.iter()),
        }
    }
}

impl<'a> Iterator for RecursibleJsonIter<'a> {
    type Item = &'a JsonValue;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Object(m) => m.next(),
            Self::Array(arr) => arr.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Object(m) => m.size_hint(),
            Self::Array(arr) => arr.size_hint(),
        }
    }
}

impl ExactSizeIterator for RecursibleJsonIter<'_> {}

impl<'a> TryFrom<&'a JsonValue> for RecursibleJson<'a> {
    type Error = &'a JsonValue;

    fn try_from(value: &'a JsonValue) -> Result<Self, Self::Error> {
        match value {
            JsonValue::Array(a) => Ok(Self::Array(a)),
            JsonValue::Object(m) => Ok(Self::Object(m)),
            value => Err(value),
        }
    }
}

pub fn get_metadata_max_depth(metadata: &GameReplayMetadata) -> usize {
    let mut max_depth = 0;

    // to_visit contains a list of things to visit and its depth
    let mut to_visit = VecDeque::new();

    to_visit.push_back((RecursibleJson::Object(&metadata.map), 0usize));

    while let Some((visitable, depth)) = to_visit.pop_front() {
        let iter = visitable.into_iter();

        for entry in iter {
            let Ok(to_add) = RecursibleJson::try_from(entry) else {
                continue;
            };

            to_visit.push_back((to_add, depth + 1));
        }

        max_depth = max_depth.max(depth);
    }

    max_depth
}
