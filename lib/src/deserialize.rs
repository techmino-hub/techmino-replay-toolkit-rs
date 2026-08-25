//! # Deserialization
//! Deserialize or parse an existing replay file or string.
//!
//! The default [`GameReplayData`] struct provides simple deserialization defaults and should
//! cover most usecases. But for e.g. streaming, use the [`ReplayDecoder`] struct instead and pass in
//! your data.
//!
//! ## Example
//! ```
//! # use libtechmino_replay::{
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
//! // Example 1: One-off parsing
//! // This is the simplest method to parse code and covers most usecases
//!
//! // .rep files are of compressed form
//! let replay_data: &[u8] = read_file("my_replay.rep");
//! let result = GameReplayData::parse_replay(replay_data, ReplayBufferKind::Compressed, None);
//!
//! // copied text replays from the Replays menu is in base64 form
//! let replay_string: &str = read_clipboard();
//! let result = GameReplayData::parse_replay(replay_string.as_bytes(), ReplayBufferKind::Base64, None);
//!
//! // Example 2: Streaming parsing, using an arbitrary stream
//! // Useful for extremely long replays
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

use core::ops::ControlFlow;

use crate::{
    InputAction,
    config::InputParseMode,
    consts::METADATA_EVENTDATA_SEPARATOR,
    errors::ReplayParseError,
    format::ReplayBufferKind,
    replay::{GameInputEvent, GameReplayData, GameReplayMetadata},
};
use alloc::{borrow::ToOwned, boxed::Box, string::String, vec::Vec};
use base64::Engine;
use libtechmino_vlq::VlqReader;
use miniz_oxide::{
    MZError,
    inflate::{TINFLStatus, stream::InflateState},
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
    /// best to give a `None` and let the library infer the input parse mode from the metadata's version
    /// string.
    ///
    /// For more information, see [`InputParseMode`].
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
    /// best to give a `None` and let the library infer the input parse mode from the metadata's version
    /// string.
    ///
    /// For more information, see [`InputParseMode`].
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
        let mut uncompressed = Vec::with_capacity(bytes.len());

        match self.preprocessor.preprocess(bytes, &mut uncompressed) {
            Ok(()) => (),
            Err(FormatError::Base64Error(e)) => return Err(ReplayParseError::Base64DecodeError(e)),
            Err(FormatError::ZlibError { status, mz_error }) => {
                return Err(ReplayParseError::ZlibDecompressError { status, mz_error });
            }
        }

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
            Self::WaitingForMetadata(metadata_decoder, override_input_mode) => {
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

                let parse_mode = 'a: {
                    if let Some(mode) = override_input_mode {
                        break 'a *mode;
                    }

                    let version = match metadata.get_version_or_raw() {
                        Some(Ok(v)) => v,
                        Some(Err(v)) => {
                            return Err(ReplayParseError::UnknownInputParseMode(Some(Err(
                                v.clone()
                            ))));
                        }
                        None => return Err(ReplayParseError::UnknownInputParseMode(None)),
                    };

                    let Some(inferred) = InputParseMode::try_infer_from_version(version) else {
                        return Err(ReplayParseError::UnknownInputParseMode(Some(Ok(
                            version.to_owned()
                        ))));
                    };

                    inferred
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
        let Self::InputDecode(decoder) = self else {
            return false;
        };

        decoder.is_finished()
    }
}

struct MetadataDecoderState {
    /// The current cumulative decompressed buffer while waiting for the
    /// [metadata-inputdata character][sep] to get produced.
    ///
    /// [sep]: METADATA_EVENTDATA_SEPARATOR
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

        let Some(newline_pos_in_input) = bytes
            .iter()
            .position(|b| *b == METADATA_EVENTDATA_SEPARATOR)
        else {
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

    /// Returns false if this struct has any leftover partial data.
    #[must_use]
    fn is_finished(&self) -> bool {
        self.vlq_reader.is_finished() && !self.expecting_action
    }
}

/// Preprocesses different replay data forms into uncompressed form.
///
/// This is a state machine. You initialize it, then as you feed in formatted
/// data, it trickles out the uncompressed/raw form of the data.
///
/// ```
/// # use libtechmino_replay::deserialize::ReplayDecoderPreprocessor;
/// # struct MyStream;
/// # impl MyStream {
/// #   fn new() -> Self { Self }
/// #   fn next(&self) -> Option<&[u8]> { None }
/// # }
/// use libtechmino_replay::config::ReplayBufferKind;
///
/// let kind = ReplayBufferKind::Uncompressed;
/// let mut preprocessor = ReplayDecoderPreprocessor::new(kind);
///
/// let mut stream: MyStream = MyStream::new();
/// let mut out_buf = Vec::new();
///
/// while let Some(chunk) = stream.next() {
///     preprocessor.preprocess(chunk, &mut out_buf)
///         .expect("preprocessing failed");
/// }
///
/// assert!(preprocessor.is_finished());
/// ```
#[instability::unstable(feature = "preprocessors")]
pub enum ReplayDecoderPreprocessor {
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
        /// The zlib decompressor state.
        decompressor: InflateState,
    },
    /// Just the zlib decompressor.
    Compressed {
        /// The zlib decompressor state.
        decompressor: InflateState,
    },
    /// No-op.
    Uncompressed,
}

impl ReplayDecoderPreprocessor {
    const SCRATCH_BUFFER_SIZE: usize = 4096;

    /// Creates a new preprocessor of the specified kind.
    #[must_use]
    #[instability::unstable(feature = "preprocessors")]
    pub fn new(kind: ReplayBufferKind) -> Self {
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

    /// Preprocesses the unprocessed data.
    ///
    /// This function only ever appends into `out_buf` and never reads or
    /// overwrites it.
    ///
    /// # Errors
    /// Returns an error if the unprocessed slice contains malformed data.
    #[instability::unstable(feature = "preprocessors")]
    pub fn preprocess(
        &mut self,
        unprocessed: &[u8],
        out_buf: &mut Vec<u8>,
    ) -> Result<(), FormatError> {
        match self {
            Self::Uncompressed => {
                out_buf.extend_from_slice(unprocessed);
                Ok(())
            }
            Self::Compressed { decompressor } => {
                Self::preprocess_compressed(decompressor, unprocessed, out_buf)?;
                Ok(())
            }
            Self::Base64 {
                b64_buffer,
                b64_buffer_len,
                decompressor,
            } => {
                Self::preprocess_b64(
                    b64_buffer,
                    b64_buffer_len,
                    decompressor,
                    unprocessed,
                    out_buf,
                )?;
                Ok(())
            }
        }
    }

    fn preprocess_compressed(
        decompressor: &mut InflateState,
        compressed_bytes: &[u8],
        out_buf: &mut Vec<u8>,
    ) -> Result<(), FormatError> {
        let mut scratch_buf = [0u8; Self::SCRATCH_BUFFER_SIZE];

        let mut compressed_bytes = compressed_bytes;

        loop {
            let res = miniz_oxide::inflate::stream::inflate(
                decompressor,
                compressed_bytes,
                &mut scratch_buf,
                miniz_oxide::MZFlush::None,
            );

            compressed_bytes = &compressed_bytes[res.bytes_consumed..];

            out_buf.extend_from_slice(&scratch_buf[..res.bytes_written]);

            match res.status {
                Ok(miniz_oxide::MZStatus::StreamEnd) => {
                    return Ok(());
                }
                Ok(_) => {
                    // We may have more output, so continue
                }
                // We need more input
                Err(miniz_oxide::MZError::Buf)
                    if decompressor.last_status() == TINFLStatus::NeedsMoreInput =>
                {
                    return Ok(());
                }
                Err(e) => {
                    return Err(FormatError::ZlibError {
                        status: decompressor.last_status(),
                        mz_error: e,
                    });
                }
            }
        }
    }

    /// Preprocess compressed-then-base64 into processed bytes.
    ///
    /// This uses interleaving to reduce intermediate heap allocations.
    fn preprocess_b64(
        b64_buffer: &mut [u8; 3],
        b64_buffer_len: &mut u8,
        decompressor: &mut InflateState,
        mut unprocessed: &[u8],
        out_buf: &mut Vec<u8>,
    ) -> Result<(), FormatError> {
        const PREDECOMP_SCRATCH_SIZE: usize =
            ReplayDecoderPreprocessor::SCRATCH_BUFFER_SIZE / 4 * 3;
        const B64_CONSUMED_PER_ITER: usize =
            base64::encoded_len(PREDECOMP_SCRATCH_SIZE, true).unwrap();
        let engine = base64::engine::general_purpose::STANDARD;

        let total_b64_len = unprocessed.len() + (*b64_buffer_len as usize);
        let processable_b64_len = if total_b64_len.is_multiple_of(4) {
            total_b64_len
        } else {
            total_b64_len.next_multiple_of(4) - 4
        };

        if processable_b64_len == 0 {
            for byte in unprocessed.iter().copied() {
                b64_buffer[*b64_buffer_len as usize] = byte;
                *b64_buffer_len += 1;
            }

            return Ok(());
        }

        // First b64 chunk, unlike the rest, may contain mix of b64
        // data from `b64_buffer` and `unprocessed`.
        // This variable contains the first b64 chunk decoded from b64.
        let first_compressed = {
            let mut first_b64 = [0u8; 4];

            let b64_buffer_len = *b64_buffer_len as usize;
            first_b64[..b64_buffer_len].clone_from_slice(&b64_buffer[..b64_buffer_len]);

            let new_data_len = 4 - b64_buffer_len;

            first_b64[b64_buffer_len..].clone_from_slice(&unprocessed[..new_data_len]);
            unprocessed = &unprocessed[new_data_len..];

            let mut first_compressed = [0u8; 3];
            engine.decode_slice_unchecked(first_b64.as_slice(), first_compressed.as_mut_slice())?;

            first_compressed
        };

        let mut predecomp_scratch = [0u8; PREDECOMP_SCRATCH_SIZE];
        let mut decompressed_scratch = [0u8; Self::SCRATCH_BUFFER_SIZE];

        // Process first b64 chunk
        if let ControlFlow::Break(res) = inflate_step(
            decompressor,
            first_compressed.as_slice(),
            decompressed_scratch.as_mut_slice(),
            out_buf,
        ) {
            // Since we're either done or errored, we need not update
            // b64 buffers other than if we want to say we're done
            if res.is_ok() {
                *b64_buffer_len = 0;
            }
            return res;
        }

        // Process the rest of the b64 chunks
        loop {
            let predecomp_len = if unprocessed.is_empty() {
                0
            } else {
                let mut consumed_len = B64_CONSUMED_PER_ITER.min(unprocessed.len());

                if !consumed_len.is_multiple_of(4) {
                    consumed_len = consumed_len.next_multiple_of(4) - 4;
                }
                debug_assert!(
                    consumed_len.is_multiple_of(4),
                    "consumed len must be a multiple of four"
                );
                debug_assert!(
                    consumed_len <= unprocessed.len(),
                    "consumed len should be at most unprocessed len"
                );

                // SAFETY: We just bounded consumed_len to be at most unprocessed.len()
                let (consumed, unproc_bind) =
                    unsafe { unprocessed.split_at_unchecked(consumed_len) };
                unprocessed = unproc_bind;

                engine.decode_slice_unchecked(consumed, predecomp_scratch.as_mut_slice())?
            };

            if let ControlFlow::Break(res) = inflate_step(
                decompressor,
                &predecomp_scratch[..predecomp_len],
                decompressed_scratch.as_mut_slice(),
                out_buf,
            ) {
                // Since we're either done or errored, we need not update
                // b64 buffers other than if we want to say we're done
                if res.is_ok() {
                    *b64_buffer_len = 0;
                }
                return res;
            }

            // We're out of input but the stream ain't over yet!
            if unprocessed.len() < 4 {
                break;
            }
        }

        #[expect(
            clippy::cast_possible_truncation,
            reason = "total_len is at most 3 more than processable_len"
        )]
        {
            *b64_buffer_len = (total_b64_len - processable_b64_len) as u8;
        }

        for (idx, item) in b64_buffer
            .iter_mut()
            .take(*b64_buffer_len as usize)
            .enumerate()
        {
            *item = unprocessed[idx];
        }

        Ok(())
    }

    /// Returns false if this struct has some leftover partial data.
    #[must_use]
    #[instability::unstable(feature = "preprocessors")]
    pub fn is_finished(&self) -> bool {
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

/// Decompress from the `predecomp_scratch` buffer temporarily into the
/// `decompression_scratch` buffer, ultimately appending it to the `out_buf` Vec.
///
/// # Returns
/// Returns `Continue` if the step finished, `Break(Ok(()))` if the stream ended,
/// or `Break(Err(e))` if an error occurred.
fn inflate_step(
    decompressor: &mut InflateState,
    mut predecomp_scratch: &[u8],
    decompression_scratch: &mut [u8],
    out_buf: &mut Vec<u8>,
) -> ControlFlow<Result<(), FormatError>> {
    loop {
        let res = miniz_oxide::inflate::stream::inflate(
            decompressor,
            predecomp_scratch,
            decompression_scratch,
            miniz_oxide::MZFlush::None,
        );

        out_buf.extend_from_slice(&decompression_scratch[..res.bytes_written]);
        predecomp_scratch = &predecomp_scratch[res.bytes_consumed..];

        match res.status {
            Ok(miniz_oxide::MZStatus::StreamEnd) => {
                return ControlFlow::Break(Ok(()));
            }
            Ok(_) => {
                // We may have more output, so continue
            }
            // We need more input
            Err(miniz_oxide::MZError::Buf)
                if decompressor.last_status() == TINFLStatus::NeedsMoreInput =>
            {
                return ControlFlow::Continue(());
            }
            Err(e) => {
                return ControlFlow::Break(Err(FormatError::ZlibError {
                    status: decompressor.last_status(),
                    mz_error: e,
                }));
            }
        }
    }
}

/// Something is wrong with the format of the given replay data.
#[derive(Debug)]
#[instability::unstable(feature = "preprocessors")]
pub enum FormatError {
    /// The given data is not valid zlib-compressed data.
    ///
    /// Or, the underlying data decoded from base64 is not valid
    /// zlib-compressed data.
    ZlibError {
        /// The status of the decompressor.
        status: TINFLStatus,
        /// An error returned by miniz.
        mz_error: MZError,
    },
    /// The given data is not valid base64.
    Base64Error(base64::DecodeError),
}

impl From<base64::DecodeError> for FormatError {
    fn from(value: base64::DecodeError) -> Self {
        Self::Base64Error(value)
    }
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

#[instability::unstable(feature = "preprocessors")]
impl core::fmt::Display for FormatError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FormatError::ZlibError { status, mz_error } => write!(
                f,
                "Zlib decompression failed (error {mz_error:?}) with status {status:?}"
            ),
            FormatError::Base64Error(decode_error) => {
                write!(f, "Base64 decode faailed ({decode_error})")
            }
        }
    }
}

/// Return the data was decoded in the last update call.
///
/// Not cumulative; if you want a full replay, you have to build it yourself.
#[must_use = "the newly-decoded data is in the `Decoded` struct"]
#[non_exhaustive]
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
        GameInputEvent, InputParseMode,
        deserialize::ReplayDecoderPreprocessor,
        format::ReplayBufferKind,
        replay::action::{InputAction, InputActionKey, InputActionKind},
        test_utils::{ByteFeeder, slightly_random_data},
    };
    use base64::Engine;
    use fastrand::Rng;
    use libtechmino_vlq::VlqData;

    const PREPROCESSOR_TRIALS: usize = 1024;

    #[test]
    fn preprocess_uncompressed() {
        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..PREPROCESSOR_TRIALS {
            let init_data = slightly_random_data(&mut rng);
            let mut feeder = ByteFeeder::new(&init_data);
            let mut result_data = Vec::new();

            let mut preprocessor = ReplayDecoderPreprocessor::new(ReplayBufferKind::Uncompressed);

            while !feeder.is_empty() {
                preprocessor
                    .preprocess(feeder.bite(&mut rng), &mut result_data)
                    .expect("preprocessor should not error");
            }

            assert_eq!(&*init_data, result_data.as_slice());
        }
    }

    #[test]
    fn preprocess_compressed() {
        let mut rng = Rng::with_seed(0x173E_C0AB_520D_8524);

        for i in 0..PREPROCESSOR_TRIALS {
            eprintln!("trial {i} of {PREPROCESSOR_TRIALS}");
            let init_data = slightly_random_data(&mut rng);

            let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&init_data, 1);

            assert_eq!(
                &*miniz_oxide::inflate::decompress_to_vec_zlib(&compressed)
                    .expect("compressed data should be valid zlib"),
                &*init_data,
                "zlib should roundtrip properly"
            );

            let mut feeder = ByteFeeder::new(&compressed);
            let mut result_data = Vec::new();

            let mut preprocessor = ReplayDecoderPreprocessor::new(ReplayBufferKind::Compressed);

            while !feeder.is_empty() {
                preprocessor
                    .preprocess(feeder.bite(&mut rng), &mut result_data)
                    .expect("preprocessor should not error");
            }

            let ReplayDecoderPreprocessor::Compressed { decompressor, .. } = preprocessor else {
                panic!("Compressed preprocessor ctor returned non-`Compressed` variant")
            };

            assert_eq!(
                decompressor.last_status(),
                TINFLStatus::Done,
                "decompressor was not done"
            );
            // assert_eq!(
            //     init_data.len(),
            //     result_data.len(),
            //     "input vs output data lengths don't match"
            // );
            assert_eq!(
                &*init_data,
                result_data.as_slice(),
                "input vs output data don't match"
            );
        }
    }

    #[test]
    fn preprocess_inner_b64() {
        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..PREPROCESSOR_TRIALS {
            let init_data = slightly_random_data(&mut rng);
            let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&init_data, 1);
            let encoded = base64::engine::general_purpose::STANDARD.encode(&compressed);

            let mut feeder = ByteFeeder::new(encoded.as_bytes());
            let mut result_data = Vec::new();

            let mut preprocessor = ReplayDecoderPreprocessor::new(ReplayBufferKind::Base64);

            while !feeder.is_empty() {
                match preprocessor {
                    ReplayDecoderPreprocessor::Base64 {
                        ref mut b64_buffer,
                        ref mut b64_buffer_len,
                        ref mut decompressor,
                        ..
                    } => {
                        ReplayDecoderPreprocessor::preprocess_b64(
                            b64_buffer,
                            b64_buffer_len,
                            decompressor,
                            feeder.bite(&mut rng),
                            &mut result_data,
                        )
                        .expect("preprocessor should not error");
                    }
                    _ => panic!("Base64 preprocessor ctor returned non-`Base64` variant"),
                }
            }

            assert_eq!(&*init_data, result_data.as_slice());
        }
    }

    #[test]
    fn preprocess_b64() {
        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..PREPROCESSOR_TRIALS {
            let init_data = slightly_random_data(&mut rng);

            let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&init_data, 1);
            let encoded = base64::engine::general_purpose::STANDARD.encode(compressed);

            let mut feeder = ByteFeeder::new(encoded.as_bytes());
            let mut result_data = Vec::new();

            let mut preprocessor = ReplayDecoderPreprocessor::new(ReplayBufferKind::Base64);

            while !feeder.is_empty() {
                preprocessor
                    .preprocess(feeder.bite(&mut rng), &mut result_data)
                    .expect("preprocessor should not error");
            }

            assert_eq!(&*init_data, result_data.as_slice());
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

    /// Tests using only the input data of the earlyinput replay, absolute mode
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
