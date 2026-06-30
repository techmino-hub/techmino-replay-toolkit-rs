use std::collections::VecDeque;

use libfuzzer_sys::arbitrary::{unstructured::Unstructured, Arbitrary};
use libtechmino_replay::{
    serialize::ReplayEncoder, GameInputEvent, GameReplayData, GameReplayMetadata, InputParseMode,
    ReplayBufferKind,
};
use serde_json::{Map as JsonMap, Value as JsonValue};

pub const MAX_METADATA_DEPTH: usize = 120;

#[derive(Debug)]
pub struct EncodeStream {
    /// The entire game replay data to encode
    total_game_data: GameReplayData,
    /// The list of indices to encode in each pass.
    ///
    /// Each `.update()` pass will insert a certain slice of the
    /// total game replay data's input list.
    ///
    /// If we are in pass `p` where `p < indices.len()`, then we will call
    /// `.update()` using `inputs[indices[p - 1]..indices[p]]`,
    /// or if p == 0, then `inputs[..indices[p]]`
    indices: Vec<usize>,
    rep_kind: ReplayBufferKind,
    compression_level: u8,
    input_mode_override: Option<InputParseMode>,
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
