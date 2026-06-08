//! # Deserialization
//! Deserialize or parse an existing replay file.
//!
//! The default [`GameReplayData`] struct provides simple deserialization defaults and should
//! cover most usecases. But for e.g. streaming, use the [`ReplayDecoder`] struct instead and pass in
//! your data.
//!
//! ## Example
//! ```
//! # use techmino_replay_toolkit::{
//! #   GameReplayData, format::ReplayBufferKind, deserialize::ReplayDecoder, GameReplayMetadata,
//! #   GameInputEvent,
//! # };
//! # fn read_file(_source: &str) -> &[u8] { &[] }
//! # fn read_clipboard() -> &'static str { "" }
//! # struct Stream;
//! # impl Stream {
//! #   fn new() -> Self {
//! #       Self
//! #   }
//! #   fn next(&mut self) -> &[u8] {
//! #       &[]
//! #   }
//! #   fn is_empty(&self) -> bool {
//! #       true
//! #   }
//! # }
//!
//! // .rep files are of compressed form
//! let replay_data: &[u8] = read_file("my_replay.rep");
//! let result = GameReplayData::parse_replay(replay_data, ReplayBufferKind::Compressed, None);
//!
//! // copied text replays from the Replays menu is in base64 form
//! let replay_string: &str = read_clipboard();
//! let result = GameReplayData::parse_replay(replay_string.as_bytes(), ReplayBufferKind::Base64, None);
//!
//! // Streaming example, using an arbitrary stream
//! // We will gradually fill in the details at it comes in
//! let mut metadata: Option<GameReplayMetadata> = None;
//! let mut inputs: Vec<GameInputEvent> = Vec::new();
//! let mut my_stream = Stream::new();
//! let mut decoder = ReplayDecoder::new(ReplayBufferKind::Compressed, None);
//!
//! while !my_stream.is_empty() {
//!     let next_chunk: &[u8] = my_stream.next();
//!     let decoded = decoder.update(next_chunk).unwrap();
//!     if let Some(m) = decoded.metadata {
//!         metadata = Some(*m);
//!     }
//!
//!     inputs.extend_from_slice(&decoded.inputs);
//! }
//! ```

use crate::{
    format::ReplayBufferKind,
    types::{GameInputEvent, GameReplayData, GameReplayMetadata, InputParseMode, ReplayParseError},
    vlq::VlqReader,
    InputAction,
};
use alloc::{borrow::Cow, boxed::Box, string::String, vec::Vec};
use base64::Engine;
use miniz_oxide::{
    inflate::{stream::InflateState, TINFLStatus},
    MZError,
};

impl GameReplayData {
    /// Converts an entire replay data into a replay data format.
    ///
    /// # Errors
    /// This function may error when the replay data is invalid or incomplete.
    ///
    /// If you have a partial replay or a replay data stream, you should manually
    /// use [`ReplayDecoder`] and feed the data yourself.
    ///
    /// # Input Parse Mode
    /// This function takes in an input parse mode override. This is often not required, but can be useful
    /// if you're targeting a mod and this library fails to infer the input parse mode from the version.
    ///
    /// Passing in the wrong input parse mode will result in nonsensical inputs, though, so it's usually
    /// best to give a `None`.
    pub fn parse_replay(
        replay_data: &[u8],
        kind: ReplayBufferKind,
        input_mode: Option<InputParseMode>,
    ) -> Result<Self, ReplayParseError> {
        let mut decoder = ReplayDecoder::new(kind, input_mode);

        let decoded_data = decoder.update(replay_data)?;

        if !decoder.is_finished() {
            if matches!(decoder.state, ReplayDecoderState::WaitingForMetadata { .. }) {
                return Err(ReplayParseError::MetadataSeparatorNotFound);
            }
            return Err(ReplayParseError::UnexpectedEnd);
        }

        let Some(metadata) = decoded_data.metadata else {
            return Err(ReplayParseError::UnexpectedEnd);
        };

        Ok(Self {
            metadata: *metadata,
            inputs: decoded_data.inputs,
        })
    }
}

impl TryFrom<&[u8]> for GameReplayMetadata {
    type Error = ReplayParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let string = String::from_utf8(Vec::from(value))?;

        Ok(serde_json::from_str(&string)?)
    }
}

/// A decoder for Techmino replays.
///
/// This decoder is NOT cumulative, you are expected to
/// create an external buffer to store its output or stream
/// it elsewhere.
///
/// # Large Size
/// This struct has a large size. To avoid stack overflow, you should
/// [`Box`] this struct if you're using this a lot or over a long period
/// of time.
pub struct ReplayDecoder {
    /// The preprocessor responsible to convert the
    /// inputted data into the same raw format to
    /// be given to the decoder.
    preprocessor: ReplayDecoderPreprocessor,
    /// The inner decoder state machine that takes in replay data
    /// in raw, uncompressed format from the preprocessor.
    state: ReplayDecoderState,
}

impl ReplayDecoder {
    /// Create a new [`ReplayDecoder`].
    ///
    /// You must give the kind of replay data you will feed this decoder.
    /// For more information, see [`ReplayBufferKind`].
    ///
    /// # Input Parse Mode
    /// This function takes in an input parse mode override. This is often not required, but can be useful
    /// if you're targeting a mod and this library fails to infer the input parse mode from the version.
    ///
    /// Passing in the wrong input parse mode will result in nonsensical inputs, though, so it's usually
    /// best to give a `None`.
    #[must_use]
    pub fn new(kind: ReplayBufferKind, input_mode: Option<InputParseMode>) -> Self {
        Self {
            state: ReplayDecoderState::WaitingForMetadata(MetadataDecoderState::new(), input_mode),
            preprocessor: ReplayDecoderPreprocessor::new(kind),
        }
    }

    /// Feed this [`ReplayDecoder`] some data.
    ///
    /// This can be used multiple times. For example, if you have
    /// a stream of bytes, you can gradually feed the bytes into this
    /// update function.
    ///
    /// # Errors
    /// This function errors when the given byte slice is invalid.
    /// This may be due to:
    /// - Incorrect replay buffer kind
    /// - Malformed replay data
    pub fn update(&mut self, bytes: &[u8]) -> Result<Decoded, ReplayParseError> {
        let uncompressed = match self.preprocessor.preprocess(bytes) {
            Ok(b) => b,
            Err(FormatError::Base64Error(e)) => return Err(ReplayParseError::Base64DecodeError(e)),
            Err(FormatError::ZlibError { status, mz_error }) => {
                return Err(ReplayParseError::ZlibDecompressError { status, mz_error })
            }
        };

        self.state.update(&uncompressed)
    }

    /// Returns true when this struct no longer has any partial data.
    ///
    /// This does NOT mean that the replay is guaranteed to have finished, this
    /// ONLY means that it's fine if the replay finished at the current state.
    ///
    /// This function may return true even though you have more data to process.
    /// But this function should return false if there isn't any more data to process.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.preprocessor.is_finished() && self.state.is_finished()
    }
}

/// The inner state machine for the replay decoder, taking
/// in the raw uncompressed replay data.
enum ReplayDecoderState {
    /// Waiting for the metadata section to finish.
    WaitingForMetadata(MetadataDecoderState, Option<InputParseMode>),
    /// Decoding inputs.
    InputDecode(InputDecoderState),
}

impl ReplayDecoderState {
    /// Run an iteration of the decoder.
    ///
    /// **`bytes` is expected to be in uncompressed/raw format.**
    fn update(&mut self, bytes: &[u8]) -> Result<Decoded, ReplayParseError> {
        match self {
            Self::WaitingForMetadata(metadata_decoder, ref override_input_mode) => {
                let res = metadata_decoder.update(bytes)?;

                let MetadataDecoderStatus::Done {
                    metadata,
                    unprocessed,
                } = res
                else {
                    return Ok(Decoded {
                        metadata: None,
                        inputs: Vec::new(),
                    });
                };

                let Some(parse_mode) = override_input_mode
                    .or_else(|| InputParseMode::try_infer_from_version(&metadata.version))
                else {
                    {
                        return Err(ReplayParseError::UnknownInputParseMode(metadata.version));
                    }
                };

                let mut input_decoder = InputDecoderState::new(parse_mode);

                let inputs = input_decoder.update(unprocessed)?;

                *self = Self::InputDecode(input_decoder);

                Ok(Decoded {
                    metadata: Some(metadata),
                    inputs,
                })
            }
            Self::InputDecode(input_decoder) => Ok(Decoded {
                metadata: None,
                inputs: input_decoder.update(bytes)?,
            }),
        }
    }

    /// Returns true when this struct no longer has any partial data.
    ///
    /// This does NOT mean that the replay is guaranteed to have finished, this
    /// ONLY means that it's fine if the replay finished at the current state.
    fn is_finished(&self) -> bool {
        let Self::InputDecode(ref decoder) = self else {
            return false;
        };

        decoder.is_finished()
    }
}

struct MetadataDecoderState {
    /// The current cumulative decompressed buffer while waiting for the
    /// newline (`\n` = `0xA`) character to get produced.
    buf: Vec<u8>,
}

impl MetadataDecoderState {
    #[must_use]
    fn new() -> Self {
        Self {
            buf: Vec::with_capacity(4096),
        }
    }

    fn update<'a>(
        &mut self,
        bytes: &'a [u8],
    ) -> Result<MetadataDecoderStatus<'a>, ReplayParseError> {
        let prev_buf_len = self.buf.len();
        self.buf.extend_from_slice(bytes);

        let Some(newline_pos_in_input) = bytes.iter().position(|b| *b == b'\n') else {
            return Ok(MetadataDecoderStatus::NotDone);
        };

        let newline_pos_in_buf = newline_pos_in_input + prev_buf_len;

        let metadata =
            serde_json::from_slice::<GameReplayMetadata>(&self.buf[..newline_pos_in_buf])?;

        let unprocessed = bytes
            .get(newline_pos_in_input + 1..)
            .unwrap_or(const { &[] });

        Ok(MetadataDecoderStatus::Done {
            metadata: Box::new(metadata),
            unprocessed,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
enum MetadataDecoderStatus<'a> {
    /// Still needs more data.
    NotDone,
    /// The metadata is ready!
    Done {
        /// The game replay metadata.
        metadata: Box<GameReplayMetadata>,
        /// The leftover non-metadata slice.
        /// This is usually a part of the input
        /// data VLQ section.
        unprocessed: &'a [u8],
    },
}

struct InputDecoderState {
    /// The VLQ reader state machine.
    vlq_reader: VlqReader,

    /// The previous frame number that was processed.
    ///
    /// Used for [`InputParseMode::Relative`].
    /// Also contains the time to pair to the next action, if `expecting_action`
    /// is on.
    prev_frame: u64,

    /// If true, the `prev_time` variable has not been matched with a corresponding
    /// action.
    expecting_action: bool,

    /// How to parse time VLQs.
    parse_mode: InputParseMode,
}

impl InputDecoderState {
    #[must_use]
    fn new(parse_mode: InputParseMode) -> Self {
        Self {
            vlq_reader: VlqReader::new(),
            prev_frame: 0,
            expecting_action: false,
            parse_mode,
        }
    }

    fn update(&mut self, vlq_bytes: &[u8]) -> Result<Vec<GameInputEvent>, ReplayParseError> {
        // Assume each input event is 3 bytes in length on average (shorter than ~273 s)
        let cap = vlq_bytes.len() / 3;

        let mut vec = Vec::with_capacity(cap);

        self.update_into_vec(vlq_bytes, &mut vec)?;

        Ok(vec)
    }

    fn update_into_vec(
        &mut self,
        vlq_bytes: &[u8],
        input_events: &mut Vec<GameInputEvent>,
    ) -> Result<(), ReplayParseError> {
        // Assume each VLQ takes up 2 bytes in length on average (shorter than ~273 s)
        let cap = vlq_bytes.len() / 2;
        let mut vlq_data_points = Vec::with_capacity(cap);

        self.vlq_reader
            .update_to_vec(vlq_bytes, &mut vlq_data_points)?;

        let mut vlqs_iter = self
            .expecting_action
            .then_some(self.prev_frame)
            .into_iter()
            .chain(vlq_data_points.drain(..).map(|v| v.value()));

        loop {
            let Some(raw_frame) = vlqs_iter.next() else {
                self.expecting_action = false;
                return Ok(());
            };

            let frame = if self.expecting_action {
                self.prev_frame
            } else {
                self.expecting_action = true;

                let frame = match self.parse_mode {
                    InputParseMode::Absolute => raw_frame,
                    InputParseMode::Relative => self.prev_frame + raw_frame,
                };

                self.prev_frame = frame;

                frame
            };

            let Some(raw_action) = vlqs_iter.next() else {
                return Ok(());
            };

            let action =
                u8::try_from(raw_action).map_err(|_| ReplayParseError::MalformedInputData {
                    raw_frame,
                    frame,
                    action: raw_action,
                })?;
            let action = InputAction::try_from(action).map_err(|_| {
                ReplayParseError::MalformedInputData {
                    raw_frame,
                    frame,
                    action: raw_action,
                }
            })?;

            let event = GameInputEvent::new(frame, action).map_err(|_| {
                ReplayParseError::MalformedInputData {
                    raw_frame,
                    frame,
                    action: raw_action,
                }
            })?;

            self.expecting_action = false;

            input_events.push(event);
        }
    }

    /// Returns false if this struct has any leftover
    /// partial data.
    #[must_use]
    fn is_finished(&self) -> bool {
        self.vlq_reader.is_finished() && !self.expecting_action
    }
}

/// Preprocesses different replay data forms into uncompressed form.
enum ReplayDecoderPreprocessor {
    /// Preprocess by decoding base64 and then
    /// decompressing it
    Base64 {
        /// b64 has this quirk where each input char/byte
        /// encodes 6 bits of data.
        /// This means 4 bytes of base64 becomes 3 bytes of data.
        /// We need to store partials here so we need to store a max
        /// of 3 bytes of base64.
        b64_buffer: [u8; 3],
        /// The amount of space in the b64 buffer that is used.
        b64_buffer_len: u8,

        /// The zlib decompressor.
        decompressor: InflateState,
    },
    /// Just the zlib decompressor.
    Compressed { decompressor: InflateState },
    /// No-op.
    Uncompressed,
}

impl ReplayDecoderPreprocessor {
    const DECOMP_BUFFER_SIZE: usize = 4096;

    fn new(kind: ReplayBufferKind) -> Self {
        match kind {
            ReplayBufferKind::Base64 => Self::Base64 {
                b64_buffer: [0u8; 3],
                b64_buffer_len: 0,
                decompressor: InflateState::new(miniz_oxide::DataFormat::Zlib),
            },
            ReplayBufferKind::Compressed => Self::Compressed {
                decompressor: InflateState::new(miniz_oxide::DataFormat::Zlib),
            },
            ReplayBufferKind::Uncompressed => Self::Uncompressed,
        }
    }

    fn preprocess<'a>(&mut self, unprocessed: &'a [u8]) -> Result<Cow<'a, [u8]>, FormatError> {
        match self {
            Self::Uncompressed => Ok(Cow::Borrowed(unprocessed)),
            Self::Compressed { decompressor } => {
                Self::preprocess_compressed(decompressor, unprocessed)
            }
            Self::Base64 {
                b64_buffer,
                b64_buffer_len,
                decompressor,
            } => {
                let compressed = Self::preprocess_b64(b64_buffer, b64_buffer_len, unprocessed)?;
                Self::preprocess_compressed(decompressor, &compressed)
            }
        }
    }

    fn preprocess_compressed(
        decompressor: &mut InflateState,
        compressed_bytes: &[u8],
    ) -> Result<Cow<'static, [u8]>, FormatError> {
        let mut out_vec = Vec::new();
        let mut out_buf = [0u8; Self::DECOMP_BUFFER_SIZE];

        let mut compressed_bytes = compressed_bytes;

        loop {
            let res = miniz_oxide::inflate::stream::inflate(
                decompressor,
                compressed_bytes,
                &mut out_buf,
                miniz_oxide::MZFlush::None,
            );

            compressed_bytes = &[];

            match res.status {
                Ok(miniz_oxide::MZStatus::StreamEnd) => {
                    out_vec.extend_from_slice(&out_buf[..res.bytes_written]);
                    return Ok(Cow::Owned(out_vec));
                }
                Ok(_) => {
                    out_vec.extend_from_slice(&out_buf[..res.bytes_written]);
                    // We may have more output, so continue
                }
                // We need more input
                Err(miniz_oxide::MZError::Buf) if res.bytes_written == 0 => {
                    return Ok(Cow::Owned(out_vec));
                }
                // Genuine error
                Err(e) => {
                    return Err(FormatError::ZlibError {
                        status: decompressor.last_status(),
                        mz_error: e,
                    });
                }
            }
        }
    }

    /// Preprocess base64 into compressed bytes.
    fn preprocess_b64(
        b64_buffer: &mut [u8; 3],
        b64_buffer_len: &mut u8,
        unprocessed: &[u8],
    ) -> Result<Vec<u8>, FormatError> {
        let engine = base64::engine::general_purpose::STANDARD;

        let total_len = unprocessed.len() + (*b64_buffer_len as usize);
        let processable_len = if total_len.is_multiple_of(4) {
            total_len
        } else {
            total_len.next_multiple_of(4) - 4
        };

        if processable_len == 0 {
            for byte in unprocessed.iter().copied() {
                b64_buffer[*b64_buffer_len as usize] = byte;
                *b64_buffer_len += 1;
            }

            return Ok(Vec::new());
        }

        let mut compressed: Vec<u8> =
            Vec::with_capacity(base64::decoded_len_estimate(processable_len));

        // The first chunk may contain a mix of `b64_buffer` and `unprocessed`
        let first_chunk: [u8; 4] = core::array::from_fn(|i| {
            #[expect(clippy::cast_possible_truncation, reason = "4 is below u8::MAX")]
            let i = i as u8;
            if let Some(unprocessed_idx) = i.checked_sub(*b64_buffer_len) {
                unprocessed[unprocessed_idx as usize]
            } else {
                b64_buffer[i as usize]
            }
        });

        if let Err(e) = engine.decode_vec(first_chunk, &mut compressed) {
            return Err(FormatError::Base64Error(e));
        }

        if processable_len == 4 {
            let unused_len = total_len - processable_len;
            debug_assert!(unused_len < 4);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "unused_len is always less than 4"
            )]
            {
                *b64_buffer_len = unused_len as u8;
            }

            for (i, item) in b64_buffer.iter_mut().enumerate().take(unused_len) {
                let unprocessed_idx = unprocessed.len() + i - unused_len;
                *item = unprocessed[unprocessed_idx];
            }

            return Ok(compressed);
        }

        // The rest may contain only `unprocessed`'s bytes
        let rest_length = processable_len - 4;
        let rest_start = usize::from(4 - *b64_buffer_len);
        let rest_end = rest_start + rest_length;

        let rest = &unprocessed[rest_start..rest_end];

        debug_assert!(rest.len().is_multiple_of(4));
        debug_assert!(unprocessed.len() >= rest_end);

        if let Err(e) = engine.decode_vec(rest, &mut compressed) {
            return Err(FormatError::Base64Error(e));
        }

        #[expect(
            clippy::cast_possible_truncation,
            reason = "total_len is at most 3 more than processable_len"
        )]
        {
            *b64_buffer_len = (total_len - processable_len) as u8;
        }

        for (idx, item) in b64_buffer
            .iter_mut()
            .take(*b64_buffer_len as usize)
            .enumerate()
        {
            *item = unprocessed[idx + rest_end];
        }

        Ok(compressed)
    }

    /// Returns false if this struct has some leftover
    /// partial data.
    fn is_finished(&self) -> bool {
        match self {
            Self::Uncompressed => true,
            Self::Compressed { decompressor } => decompressor.last_status() == TINFLStatus::Done,
            Self::Base64 {
                b64_buffer: _,
                b64_buffer_len,
                decompressor,
            } => decompressor.last_status() == TINFLStatus::Done && *b64_buffer_len == 0,
        }
    }
}

/// Something is wrong with the format of the given replay data.
#[derive(Debug)]
enum FormatError {
    /// The given data is not valid zlib-compressed data.
    ///
    /// Or, the underlying data decoded from base64 is not valid
    /// zlib-compressed data.
    ZlibError {
        status: TINFLStatus,
        mz_error: MZError,
    },
    /// The given data is not valid base64.
    Base64Error(base64::DecodeError),
}

impl From<FormatError> for ReplayParseError {
    fn from(value: FormatError) -> Self {
        match value {
            FormatError::ZlibError { status, mz_error } => {
                Self::ZlibDecompressError { status, mz_error }
            }
            FormatError::Base64Error(decode_error) => Self::Base64DecodeError(decode_error),
        }
    }
}

/// Return the data was decoded in the last update call.
///
/// Not cumulative; if you want a full replay, you have to build it yourself.
#[must_use = "the newly-decoded data is in the `Decoded` struct"]
pub struct Decoded {
    /// The metadata, if it was decoded in this update call.
    ///
    /// The metadata will only ever be returned once! Every decode return
    /// after the first `Some(...)` will result in `None`.
    pub metadata: Option<Box<GameReplayMetadata>>,

    /// The inputs decoded in this update call.
    pub inputs: Vec<GameInputEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        action::{InputAction, InputActionKey, InputActionKind},
        deserialize::ReplayDecoderPreprocessor,
        format::ReplayBufferKind,
        tests::{slightly_random_data, ByteFeeder},
        vlq::VlqData,
        GameInputEvent, InputParseMode,
    };
    use base64::Engine;
    use fastrand::Rng;

    const PREPROCESSOR_TRIALS: usize = 1024;

    #[test]
    fn preprocess_uncompressed() {
        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..PREPROCESSOR_TRIALS {
            let init_data = slightly_random_data(&mut rng);
            let mut feeder = ByteFeeder::new(init_data.as_slice());
            let mut result_data = Vec::new();

            let mut preprocessor = ReplayDecoderPreprocessor::new(ReplayBufferKind::Uncompressed);

            while !feeder.is_empty() {
                let out = preprocessor
                    .preprocess(feeder.bite(&mut rng))
                    .expect("preprocessor should not error");
                result_data.extend_from_slice(&out);
            }

            assert_eq!(init_data.as_slice(), result_data.as_slice());
        }
    }

    #[test]
    fn preprocess_compressed() {
        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..PREPROCESSOR_TRIALS {
            let init_data = slightly_random_data(&mut rng);

            let compressed = miniz_oxide::deflate::compress_to_vec_zlib(init_data.as_slice(), 1);

            let mut feeder = ByteFeeder::new(&compressed);
            let mut result_data = Vec::new();

            let mut preprocessor = ReplayDecoderPreprocessor::new(ReplayBufferKind::Compressed);

            while !feeder.is_empty() {
                let out = preprocessor
                    .preprocess(feeder.bite(&mut rng))
                    .expect("preprocessor should not error");
                result_data.extend_from_slice(&out);
            }

            assert_eq!(init_data.as_slice(), result_data.as_slice());
        }
    }

    #[test]
    fn preprocess_inner_b64() {
        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..PREPROCESSOR_TRIALS {
            let init_data = slightly_random_data(&mut rng);

            let encoded = base64::engine::general_purpose::STANDARD.encode(init_data);

            let mut feeder = ByteFeeder::new(encoded.as_bytes());
            let mut result_data = Vec::new();

            let mut preprocessor = ReplayDecoderPreprocessor::new(ReplayBufferKind::Base64);

            while !feeder.is_empty() {
                match preprocessor {
                    ReplayDecoderPreprocessor::Base64 {
                        ref mut b64_buffer,
                        ref mut b64_buffer_len,
                        ..
                    } => {
                        let out = ReplayDecoderPreprocessor::preprocess_b64(
                            b64_buffer,
                            b64_buffer_len,
                            feeder.bite(&mut rng),
                        )
                        .expect("preprocessor should not error");
                        result_data.extend_from_slice(&out);
                    }
                    _ => unreachable!(),
                }
            }

            assert_eq!(init_data.as_slice(), result_data.as_slice());
        }
    }

    #[test]
    fn preprocess_b64() {
        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..PREPROCESSOR_TRIALS {
            let init_data = slightly_random_data(&mut rng);

            let compressed = miniz_oxide::deflate::compress_to_vec_zlib(init_data.as_slice(), 1);
            let encoded = base64::engine::general_purpose::STANDARD.encode(compressed);

            let mut feeder = ByteFeeder::new(encoded.as_bytes());
            let mut result_data = Vec::new();

            let mut preprocessor = ReplayDecoderPreprocessor::new(ReplayBufferKind::Base64);

            while !feeder.is_empty() {
                let out = preprocessor
                    .preprocess(feeder.bite(&mut rng))
                    .expect("preprocessor should not error");
                result_data.extend_from_slice(&out);
            }

            assert_eq!(init_data.as_slice(), result_data.as_slice());
        }
    }

    /// Tests using only the input data of the earlyinput replay, relative mode
    #[test]
    fn earlyinput_rel_input_test() {
        const ATTEMPTS: usize = 1_000_000;

        let parse_mode = InputParseMode::Relative;
        let earlyinput_to_encode = [
            GameInputEvent::new(
                1,
                InputAction {
                    kind: InputActionKind::Press,
                    key: InputActionKey::MoveLeft,
                },
            )
            .expect("input should be valid"),
            GameInputEvent::new(
                179,
                InputAction {
                    kind: InputActionKind::Press,
                    key: InputActionKey::MoveLeft,
                },
            )
            .expect("input should be valid"),
        ];

        let mut earlyinput_bytes: Vec<u8> = Vec::new();

        let mut prev_frame = 0;
        for input in earlyinput_to_encode {
            let (frame, action) = (input.frame(), input.action());
            let frame_vlq = VlqData::from_value(frame - prev_frame)
                .expect("frame # should be in valid vlq range");
            prev_frame = frame;
            earlyinput_bytes.extend_from_slice(frame_vlq.as_slice());
            earlyinput_bytes.push(action.into());
        }

        eprint!("earlyinput_bytes: [0x");
        for byte in &earlyinput_bytes {
            eprint!("{byte:02X}_");
        }
        eprintln!("]");

        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        let mut earlyinput_decoded = Vec::with_capacity(2);

        for i in 1..=ATTEMPTS {
            let mut feeder = ByteFeeder::new(&earlyinput_bytes);
            let mut decoder = InputDecoderState::new(parse_mode);
            earlyinput_decoded.clear();

            while !feeder.is_empty() {
                decoder
                    .update_into_vec(feeder.bite(&mut rng), &mut earlyinput_decoded)
                    .expect("failed to decode replay");
            }

            assert_eq!(
                earlyinput_decoded, earlyinput_to_encode,
                "decode mismatch on attempt {i}"
            );
            eprintln!("attempt {i} succeeded");
        }
    }

    #[test]
    fn earlyinput_abs_input_test() {
        const ATTEMPTS: usize = 1_000_000;

        let parse_mode = InputParseMode::Absolute;
        let earlyinput_to_encode = [
            GameInputEvent::new(
                1,
                InputAction {
                    kind: InputActionKind::Press,
                    key: InputActionKey::MoveLeft,
                },
            )
            .expect("input should be valid"),
            GameInputEvent::new(
                179,
                InputAction {
                    kind: InputActionKind::Press,
                    key: InputActionKey::MoveLeft,
                },
            )
            .expect("input should be valid"),
        ];

        let mut earlyinput_bytes: Vec<u8> = Vec::new();

        for input in earlyinput_to_encode {
            let (frame, action) = (input.frame(), input.action());
            let frame_vlq =
                VlqData::from_value(frame).expect("frame # should be in valid vlq range");
            earlyinput_bytes.extend_from_slice(frame_vlq.as_slice());
            earlyinput_bytes.push(action.into());
        }

        eprint!("earlyinput_bytes: [0x");
        for byte in &earlyinput_bytes {
            eprint!("{byte:02X}_");
        }
        eprintln!("]");

        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        let mut earlyinput_decoded = Vec::with_capacity(2);

        for i in 1..=ATTEMPTS {
            let mut feeder = ByteFeeder::new(&earlyinput_bytes);
            let mut decoder = InputDecoderState::new(parse_mode);
            earlyinput_decoded.clear();

            while !feeder.is_empty() {
                decoder
                    .update_into_vec(feeder.bite(&mut rng), &mut earlyinput_decoded)
                    .expect("failed to decode replay");
            }

            assert_eq!(
                earlyinput_decoded, earlyinput_to_encode,
                "decode mismatch on attempt {i}"
            );
            eprintln!("attempt {i} succeeded");
        }
    }
}
