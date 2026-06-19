#![no_main]

use libfuzzer_sys::{
    arbitrary::{unstructured::Unstructured, Arbitrary},
    fuzz_target,
};
use libtechmino_replay::{
    serialize::ReplayEncoder, GameInputEvent, GameReplayData, GameReplayMetadata, InputParseMode,
    ReplayBufferKind,
};

#[derive(Debug)]
struct EncodeStream {
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

    fn test(&self) -> Result<Vec<u8>, libtechmino_replay::ReplaySerializeError> {
        let mut encoder = ReplayEncoder::new(self.rep_kind, self.compression_level);
        let mut output = encoder.feed_metadata(self.metadata(), self.input_mode_override)?;

        for pass in 0..self.indices.len() {
            let lower_bound = pass.checked_sub(1).map(|p| self.indices[p]).unwrap_or(0);
            let upper_bound = self.indices[pass];
            let input_slice = &self.inputs()[lower_bound..upper_bound];

            encoder.feed_input_data(input_slice, &mut output)?;
        }

        encoder.finish(&mut output)?;

        todo!();
    }
}

impl<'a> Arbitrary<'a> for EncodeStream {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let rep_kind = ReplayBufferKind::arbitrary(u)?;
        let compression_level = u.int_in_range(0u8..=10u8)?;
        let input_mode_override = u.arbitrary()?;

        let total_game_data = GameReplayData::arbitrary(u)?;

        let input_count = total_game_data.inputs.len();

        let mut indices = Vec::new();
        let mut prev_index = 0;

        while prev_index < input_count {
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

fuzz_target!(|data: EncodeStream| {
    let _result = data.test();
});
