//! # Serialization
//! Serialize or save replay data into raw bytes readable by the game.
//!
//! The default [`GameReplayData`] struct provides simple serialization defaults and should
//! cover most usecases. But for e.g. streaming, use the [`ReplayEncoder`] struct instead and
//! pass in your data.
//!
//! ## Example
//! ```
//! # use std::collections::HashMap;
//! # use techmino_replay_toolkit::{
//! #   GameReplayData, format::ReplayBufferKind, serialize::ReplayEncoder, GameReplayMetadata,
//! #   GameInputEvent, PlayerSettings, InputParseMode
//! # };
//! # struct Stream;
//! # impl Stream {
//! #   fn new() -> Self {
//! #       Self
//! #   }
//! #   fn next(&mut self) -> &[GameInputEvent] {
//! #       &[]
//! #   }
//! #   fn is_empty(&self) -> bool {
//! #       true
//! #   }
//! # }
//!
//! let metadata = GameReplayMetadata {
//!     // ...
//! #   date: "".into(),
//! #   mode: "".into(),
//! #   mods: None,
//! #   player: "".into(),
//! #   nonstandard: HashMap::new(),
//! #   private: None,
//! #   seed: 0,
//! #   setting: PlayerSettings {
//! #       das: None,
//! #       arr: None,
//! #       atk_fx: None,
//! #       bag_line: None,
//! #       block: None,
//! #       center: None,
//! #       clear_fx: None,
//! #       dascut: None,
//! #       drop_fx: None,
//! #       dropcut: None,
//! #       face: None,
//! #       ft_lock: None,
//! #       ghost: None,
//! #       grid: None,
//! #       high_cam: None,
//! #       ihs: None,
//! #       ims: None,
//! #       irs: None,
//! #       irscut: None,
//! #       lock_fx: None,
//! #       move_fx: None,
//! #       next_pos: None,
//! #       nonstandard: HashMap::new(),
//! #       rs: None,
//! #       score: None,
//! #       sdarr: None,
//! #       sddas: None,
//! #       shake_fx: None,
//! #       skin: None,
//! #       smooth: None,
//! #       splash_fx: None,
//! #       text: None,
//! #       warn: None,
//! #   },
//! #   tas_used: None,
//! #   version: "".into(),
//! };
//! let inputs: Vec<GameInputEvent> = vec![
//!     // ...
//! ];
//!
//! // Default serialization
//! let replay = GameReplayData { inputs, metadata: metadata.clone() };
//!
//! let rep_file = replay.serialize_to_compressed(Some(InputParseMode::Relative), 1);
//! let copiable_b64 = replay.serialize_to_base64(Some(InputParseMode::Relative), 1);
//!
//! // Streaming serialization
//! let mut input_stream = Stream::new();
//! let mut encoder = ReplayEncoder::new(ReplayBufferKind::Compressed, 1);
//! let mut replay_bytes: Vec<u8> = encoder.feed_metadata(&metadata, Some(InputParseMode::Relative)).unwrap();
//!
//! while !input_stream.is_empty() {
//!     let inputs: &[GameInputEvent] = input_stream.next();
//!     encoder.feed_input_data(inputs, &mut replay_bytes).unwrap();
//! }
//!
//! encoder.finish(&mut replay_bytes).unwrap();
//! ```

use crate::{
    format::ReplayBufferKind,
    types::{
        GameInputEvent, GameReplayData, GameReplayMetadata, InputParseMode, ReplaySerializeError,
    },
    vlq::VlqData,
};
use alloc::vec::Vec;
use ascii::AsciiString;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use core::ops::ControlFlow;
use miniz_oxide::{
    deflate::{
        core::{compress, CompressorOxide, TDEFLFlush, TDEFLStatus},
        CompressionLevel,
    },
    DataFormat,
};

impl GameReplayData {
    /// Sort the inputs so that they are sorted by time.
    ///
    /// This can be necessary sometimes as serializing the replay (e.g., into base64)
    /// requires that the inputs are sorted for the algorithm to work properly.
    pub fn sort_inputs(&mut self) {
        self.inputs.sort_by_key(|i| i.frame());
    }

    /// Serialize into a raw, uncompressed byte array.
    ///
    /// This function serializes the `GameReplayData` into a raw, uncompressed byte array.
    ///
    /// This will not be playable by the game as the game automatically compresses and decompresses the data.\
    /// For serializing the data into the `.rep` file format used by the game's saved replays, use
    /// [`serialize_to_compressed`][GameReplayData::serialize_to_compressed] instead.\
    /// For serializing the data into a copiable text/base64 format, use
    /// [`serialize_to_base64`][GameReplayData::serialize_to_base64] instead.
    ///
    /// Note that the serialization algorithm requires that the inputs in the replay are sorted to time.\
    /// If this isn't always the case, consider calling [`sort_inputs`][GameReplayData::sort_inputs] before calling this function,
    /// otherwise an [`UnsortedInput`][ReplaySerializeError::UnsortedInput] error will be returned.
    ///
    /// # Input Parse Mode
    /// This function takes in an input parse mode override. This is often not required, but can be useful
    /// if you're targeting a mod and this library fails to infer the input parse mode from the version.
    ///
    /// Passing in the wrong input parse mode will result in nonsensical inputs, though, so it's usually
    /// best to give a `None`.
    ///
    /// # Errors
    /// For more information, refer to [`ReplaySerializeError`]
    pub fn serialize_to_raw(
        &self,
        input_mode: Option<InputParseMode>,
    ) -> Result<Vec<u8>, ReplaySerializeError> {
        let mut encoder = ReplayEncoder::new(ReplayBufferKind::Uncompressed, 0);

        let mut output = encoder.feed_metadata(&self.metadata, input_mode)?;

        encoder.feed_input_data(&self.inputs, &mut output)?;

        Ok(output)
    }

    /// Serialize into a compressed byte array used by the game.
    ///
    /// This data format is used by the game in the form of `.rep` files that are placed in
    /// the `replays/` directory of the game's save directory.\
    /// For serializing the data into a copiable text/base64 format, use
    /// [`serialize_to_base64`][GameReplayData::serialize_to_base64] instead.\
    /// `FOr` serializing the data into a raw, uncompressed byte array form, use
    /// [`serialize_to_raw`][GameReplayData::serialize_to_raw] instead.
    ///
    /// Note that the serialization algorithm requires that the inputs in the replay are sorted to time.\
    /// If this isn't always the case, consider calling [`sort_inputs`][GameReplayData::sort_inputs] before calling this function,
    /// otherwise an [`UnsortedInput`][ReplaySerializeError::UnsortedInput] error will be returned.
    ///
    /// # Input Parse Mode
    /// This function takes in an input parse mode override. This is often not required, but can be useful
    /// if you're targeting a mod and this library fails to infer the input parse mode from the version.
    ///
    /// Passing in the wrong input parse mode will result in nonsensical inputs, though, so it's usually
    /// best to give a `None`.
    ///
    /// # Compression Level
    /// You can choose how hard to try to compress the output using zlib. The default is usually 7.
    /// For more information, see [`miniz_oxide::deflate::CompressionLevel`].
    ///
    /// # Errors
    /// For more information, refer to [`ReplaySerializeError`]
    pub fn serialize_to_compressed(
        &self,
        input_mode: Option<InputParseMode>,
        compression_level: u8,
    ) -> Result<Vec<u8>, ReplaySerializeError> {
        let mut encoder = ReplayEncoder::new(ReplayBufferKind::Compressed, compression_level);

        let mut output = encoder.feed_metadata(&self.metadata, input_mode)?;

        encoder.feed_input_data(&self.inputs, &mut output)?;

        Ok(output)
    }

    /// Serialize into a copiable text-based base64 format.
    ///
    /// This data format is used by the game for importing/exporting replays.
    /// For serializing the data into the `.rep` file format used by the game's saved replays, use
    /// [`serialize_to_compressed`][GameReplayData::serialize_to_compressed] instead.\
    /// `FOr` serializing the data into a raw, uncompressed byte array form, use
    /// [`serialize_to_raw`][GameReplayData::serialize_to_raw] instead.
    ///
    /// Note that the serialization algorithm requires that the inputs in the replay are sorted to time.\
    /// If this isn't always the case, consider calling [`sort_inputs`][GameReplayData::sort_inputs] before calling this function,
    /// otherwise an [`UnsortedInput`][ReplaySerializeError::UnsortedInput] error will be returned.
    ///
    /// # Input Parse Mode
    /// This function takes in an input parse mode override. This is often not required, but can be useful
    /// if you're targeting a mod and this library fails to infer the input parse mode from the version.
    ///
    /// Passing in the wrong input parse mode will result in nonsensical inputs, though, so it's usually
    /// best to give a `None`.
    ///
    /// # Compression Level
    /// You can choose how hard to try to compress the output using zlib. The default is usually 7.
    /// For more information, see [`miniz_oxide::deflate::CompressionLevel`].
    ///
    /// # Errors
    /// For more information, refer to [`ReplaySerializeError`]
    pub fn serialize_to_base64(
        &self,
        input_mode: Option<InputParseMode>,
        compression_level: u8,
    ) -> Result<AsciiString, ReplaySerializeError> {
        let mut encoder = ReplayEncoder::new(ReplayBufferKind::Base64, compression_level);

        let mut output = encoder.feed_metadata(&self.metadata, input_mode)?;

        encoder.feed_input_data(&self.inputs, &mut output)?;

        Ok(unsafe { AsciiString::from_ascii_unchecked(output) })
    }
}

/// An encoder for Techmino replays.
///
/// This encoder is NOT cumulative, you are expected to
/// create an external buffer to store its output or stream
/// it elsewhere.
pub struct ReplayEncoder {
    /// The inner state machine for the encoder, that outputs into
    /// raw uncompressed form.
    state: ReplayEncoderState,
    /// The postprocessor to remux the uncompressed replay data
    /// into compressed binary or base64 forms.
    postprocessor: ReplayEncoderPostprocessor,
}

impl ReplayEncoder {
    /// Creates a new [`ReplayEncoder`] instance.
    ///
    /// # Compression Level
    /// You can choose how hard to try to compress the output using zlib. The default is usually 7.
    /// For more information, see [`miniz_oxide::deflate::CompressionLevel`].
    ///
    /// For uncompressed replays, this parameter is ignored.
    ///
    /// # Next Steps
    /// After creating the [`ReplayEncoder`], start by feeding it some metadata to serialize \
    /// using [`feed_metadata`]. Note that that step can only be done once per encoding since
    /// there's only one metadata segment in the replay structure.
    #[must_use]
    pub fn new(rep_kind: ReplayBufferKind, compression_level: u8) -> Self {
        Self {
            state: ReplayEncoderState::DEFAULT,
            postprocessor: ReplayEncoderPostprocessor::new(rep_kind, compression_level),
        }
    }

    /// Feeds some metadata into this [`ReplayEncoder`].
    ///
    /// # Input Parse Mode
    /// This function takes in an input parse mode override. This is often not required, but can be useful
    /// if you're targeting a mod and this library fails to infer the input parse mode from the version.
    ///
    /// Passing in the wrong input parse mode will result in nonsensical inputs, though, so it's usually
    /// best to give a `None`.
    ///
    /// # Returns
    /// If this function succeeds, returns a `Vec` of encoded replay bytes. The form of this
    /// depends on the specific replay kind you chose. In the case of [`ReplayBufferKind::Base64`],
    /// the output is guaranteed to be a valid UTF-8 string.
    ///
    /// Note that the output is still incomplete, and may not be a valid replay yet.
    ///
    /// # Errors
    /// This function errors if the metadata is invalid or if the encoder isn't expecting metadata
    /// (i.e., it's already been given metadata and is now expecting input data).
    ///
    /// # Next Steps
    /// After feeding the [`ReplayEncoder`] metadata, the last step is to feed it input data
    /// using [`feed_input_data`]. Note that unlike feeding metadata, you can feed input data
    /// in multiple batches.
    pub fn feed_metadata(
        &mut self,
        metadata: &GameReplayMetadata,
        input_mode: Option<InputParseMode>,
    ) -> Result<Vec<u8>, ReplaySerializeError> {
        self.state.feed_metadata(metadata, input_mode)
    }

    /// Feeds some input data into this [`ReplayEncoder`].
    ///
    /// Any output will be appended into the given `output` `Vec`. \
    /// The `output` `Vec` doesn't need to be filled or anything, it just needs to be a `Vec<u8>`
    /// to place any output.
    ///
    /// # Errors
    /// This function errors if the input data is invalid (e.g., unsorted), or if the encoder
    /// isn't expecting input data right now (i.e., it's not yet been given metadata).
    ///
    /// # Requirements
    /// This function requires the input data to be sorted, otherwise returns an error.
    /// This function also requires that metadata has been fed to the [`ReplayEncoder`] using
    /// [`feed_metadata`][Self::feed_metadata].
    ///
    /// # Repeatable
    /// You can safely call this function multiple times with chunks of input data.
    ///
    /// # Next Steps
    /// After you have given all your input data into the encoder, you'll have to finish
    /// up by calling the [`finish`][Self::finish] function.
    pub fn feed_input_data(
        &mut self,
        mut input_data: &[GameInputEvent],
        output: &mut Vec<u8>,
    ) -> Result<(), ReplaySerializeError> {
        let mut raw_bytes_buf = [0u8; 2048];

        while !input_data.is_empty() {
            let (inputs_processed, bytes_outputted) =
                self.state.feed_input_data(input_data, &mut raw_bytes_buf)?;

            let raw_bytes_slice = &raw_bytes_buf[..bytes_outputted];

            self.postprocessor
                .postprocess_into_vec(raw_bytes_slice, output)?;

            input_data = &input_data[inputs_processed..];
        }

        Ok(())
    }

    /// Finishes up the replay.
    ///
    /// Any output will be appended into the given `output` `Vec`. \
    /// The `output` `Vec` doesn't need to be filled or anything, it just needs to be a `Vec<u8>`
    /// to place any output.
    ///
    /// # Errors
    /// This function errors if something went wrong while compressing.
    ///
    /// # Requirements
    /// This function requires the metadata to be filled, and for this encoder
    /// to not be previously finished.
    pub fn finish(&mut self, output: &mut Vec<u8>) -> Result<(), ReplaySerializeError> {
        self.postprocessor.finish_into_vec(output)?;

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ReplayEncoderState {
    WaitingForMetadata,
    InputData {
        prev_frame: u64,
        parse_mode: InputParseMode,
    },
}

impl ReplayEncoderState {
    /// The starting replay encoder state.
    const DEFAULT: Self = Self::WaitingForMetadata;

    /// Tries to encode metadata.
    ///
    /// # Returns
    /// If the operation succeeds, returns bytes consisting of the serialized metadata
    /// *and the separator between metadata and input data* in raw format, to be given
    /// to the postprocessors.
    ///
    /// # Errors
    /// This function errors if:
    /// - The current encoder state is not expecting metadata
    /// - The serialization failed
    /// - The input parse mode could not be inferred from the metadata's game version
    ///   and there was no override
    fn feed_metadata(
        &mut self,
        metadata: &GameReplayMetadata,
        parse_mode_override: Option<InputParseMode>,
    ) -> Result<Vec<u8>, ReplaySerializeError> {
        if !matches!(self, Self::WaitingForMetadata) {
            return Err(ReplaySerializeError::InvalidOperation);
        }

        let parse_mode = match parse_mode_override {
            Some(m) => m,
            None => InputParseMode::try_infer_from_version(&metadata.version).ok_or_else(|| {
                ReplaySerializeError::UnknownInputParseMode(metadata.version.clone())
            })?,
        };

        *self = Self::InputData {
            prev_frame: 0,
            parse_mode,
        };

        let mut vec = serde_json::to_vec(&metadata)?;

        vec.push(b'\n');

        Ok(vec)
    }

    /// Tries to encode input data into the given output buffer.
    ///
    /// # Returns
    /// Returns a tuple containing:
    /// - How many input events were processed.
    /// - The current length/usage of the output buffer.\
    ///   This is the first "unused" byte of the output buffer.
    ///
    /// # Errors
    /// This function errors if:
    /// - The current encoder state is not expecting input data
    /// - The input data is not sorted
    ///
    /// # Remarks
    /// This function does not allocate any more buffers, so make sure to check
    /// that all the inputs have been consumed, by checking that the outputted
    /// index equals the length of the input data. If it does not match, then
    /// that means the input data was only partially processed, in which case,
    /// process the partial outputted buffer if necessary, and then call this function
    /// again *with the already-processed data removed*.
    fn feed_input_data(
        &mut self,
        input_data: &[GameInputEvent],
        output_buffer: &mut [u8],
    ) -> Result<(usize, usize), ReplaySerializeError> {
        let Self::InputData {
            prev_frame,
            ref parse_mode,
        } = self
        else {
            return Err(ReplaySerializeError::InvalidOperation);
        };
        let parse_mode = *parse_mode;

        let mut inputs_processed = 0;
        let mut output_idx = 0;

        for &input in input_data {
            let res = Self::feed_input_data_inner(
                prev_frame,
                parse_mode,
                input,
                &mut output_buffer[output_idx..],
            )?;
            let ControlFlow::Continue(bytes_written) = res else {
                break;
            };

            output_idx += bytes_written;
            inputs_processed += 1;
        }

        Ok((inputs_processed, output_idx))
    }

    /// Processes an individual input into the output buffer.
    ///
    /// The output buffer is expected to be pre-sliced; that is, this function will try to copy
    /// starting from the zeroth element of the slice. If this is not desired, feed this function
    /// something like `&mut output_buffer[starting_idx..]` instead.
    ///
    /// # Returns
    /// If there is enough space in the output buffer, returns the amount of bytes written in
    /// `Ok(ControlFlow::Continue(bytes))`.
    /// If there is not enough space, nothing will be written and `Ok(ControlFlow::Break(())` will
    /// be returned instead.
    /// If the input data is unsorted, nothing will be written and `Err` will
    /// be returned instead.
    fn feed_input_data_inner(
        prev_frame: &mut u64,
        parse_mode: InputParseMode,
        input: GameInputEvent,
        output_buffer: &mut [u8],
    ) -> Result<ControlFlow<(), usize>, ReplaySerializeError> {
        let real_frame = input.frame();

        let encoded_frame = match parse_mode {
            InputParseMode::Absolute => real_frame,
            InputParseMode::Relative => real_frame.checked_sub(*prev_frame).ok_or({
                ReplaySerializeError::UnsortedInput {
                    prev_time: *prev_frame,
                    unsorted_time: real_frame,
                }
            })?,
        };

        // GameInputEvent is more restrictive than VlqData's requirements
        // (2^54 < 2^56).  This will never panic
        let frame_vlq = VlqData::from_value(encoded_frame).unwrap();
        let action = u8::from(input.action());

        let frame_len = frame_vlq.len().get() as usize;
        let required_len = frame_len + 1;

        if required_len > output_buffer.len() {
            return Ok(ControlFlow::Break(()));
        }

        if parse_mode == InputParseMode::Relative {
            *prev_frame = real_frame;
        }

        output_buffer[..frame_len].copy_from_slice(frame_vlq.as_slice());
        output_buffer[frame_len] = action;

        Ok(ControlFlow::Continue(required_len))
    }
}

enum ReplayEncoderPostprocessor {
    /// Compress then encode to base64
    Base64 {
        /// We can convert every three bytes into 4 base64 chars.
        /// We can have a leftover of up to two bytes in the byte array.
        b64_scratch_buffer: [u8; 2],
        /// The amount of space in the b64 scratch buffer that is currently used.
        b64_scratch_buffer_len: u8,

        /// The zlib compressor.
        compressor: CompressorOxide,
    },
    /// Just compress into binary
    Compressed {
        /// The zlib compressor.
        compressor: CompressorOxide,
    },
    /// No-op.
    Uncompressed,
}

impl ReplayEncoderPostprocessor {
    /// The size for a lone temporary buffer.
    const TEMP_BUFFER_SIZE: usize = 4096;
    /// The size for a temporary buffer containing compressed bytes when
    /// it's not the sole buffer.
    const COMPRESSED_BUFFER_SIZE: usize = ReplayEncoderPostprocessor::TEMP_BUFFER_SIZE / 2;
    /// The size for a temporary buffer containing base64 bytes when it's
    /// not the sole buffer.
    const BASE64_BUFFER_SIZE: usize =
        base64::encoded_len(Self::COMPRESSED_BUFFER_SIZE, true).unwrap() + 4;

    /// Creates a new postprocessor of a specific kind.
    ///
    /// The compression level dictates, for compressed or base64 formats,
    /// how hard to try to compress the replay data. For more information,
    /// see [`miniz_oxide::deflate::CompressionLevel`].
    ///
    /// The compression level is ignored for uncompressed formats.
    fn new(kind: ReplayBufferKind, compression_level: u8) -> Self {
        match kind {
            ReplayBufferKind::Base64 => {
                let mut compressor = CompressorOxide::with_format_and_level(
                    DataFormat::Zlib,
                    CompressionLevel::DefaultCompression,
                );
                compressor.set_compression_level_raw(compression_level);

                Self::Base64 {
                    b64_scratch_buffer: [0u8; 2],
                    b64_scratch_buffer_len: 0,
                    compressor,
                }
            }
            ReplayBufferKind::Compressed => {
                let mut compressor = CompressorOxide::with_format_and_level(
                    DataFormat::Zlib,
                    CompressionLevel::DefaultCompression,
                );
                compressor.set_compression_level_raw(compression_level);

                Self::Compressed { compressor }
            }
            ReplayBufferKind::Uncompressed => Self::Uncompressed,
        }
    }

    fn postprocess_into_vec(
        &mut self,
        raw: &[u8],
        out_bytes: &mut Vec<u8>,
    ) -> Result<(), TDEFLStatus> {
        match self {
            Self::Base64 {
                compressor,
                b64_scratch_buffer,
                b64_scratch_buffer_len,
            } => Self::postprocess_b64(
                compressor,
                b64_scratch_buffer,
                b64_scratch_buffer_len,
                raw,
                out_bytes,
            ),
            Self::Compressed { compressor } => {
                Self::postprocess_compression(compressor, raw, out_bytes)?;
                Ok(())
            }
            Self::Uncompressed => {
                out_bytes.extend_from_slice(raw);
                Ok(())
            }
        }
    }

    /// Compress the bytes into the output.
    fn postprocess_compression(
        compressor: &mut CompressorOxide,
        raw: &[u8],
        compression_output: &mut Vec<u8>,
    ) -> Result<(), TDEFLStatus> {
        let mut raw = raw;
        let mut buf = [0u8; Self::TEMP_BUFFER_SIZE];

        loop {
            let (status, raw_idx, buf_idx) = compress(compressor, raw, &mut buf, TDEFLFlush::None);

            match status {
                TDEFLStatus::Okay => (),
                TDEFLStatus::Done => unreachable!(), // we're not flushing
                TDEFLStatus::BadParam | TDEFLStatus::PutBufFailed => return Err(status),
            }

            raw = raw.get(raw_idx..).unwrap_or(const { &[] });
            compression_output.extend_from_slice(&buf[..buf_idx]);

            if raw.is_empty() || buf_idx < buf.len() {
                return Ok(());
            }
        }
    }

    /// Compress the uncompressed bytes and encode those compressed bytes
    /// into base64.
    fn postprocess_b64(
        compressor: &mut CompressorOxide,
        b64_scratch_buffer: &mut [u8; 2],
        b64_scratch_buffer_len: &mut u8,
        raw: &[u8],
        b64_output: &mut Vec<u8>,
    ) -> Result<(), TDEFLStatus> {
        let mut raw = raw;
        let mut compressed_buf = [0u8; Self::COMPRESSED_BUFFER_SIZE];
        let mut b64_out_buf = [0u8; Self::BASE64_BUFFER_SIZE];

        loop {
            let (status, raw_idx, cmp_buf_idx) =
                compress(compressor, raw, &mut compressed_buf, TDEFLFlush::None);

            match status {
                TDEFLStatus::Okay => (),
                TDEFLStatus::Done => unreachable!(), // we're not flushing
                TDEFLStatus::BadParam | TDEFLStatus::PutBufFailed => return Err(status),
            }

            raw = raw.get(raw_idx..).unwrap_or(const { &[] });

            let compressed_slice = &compressed_buf[..cmp_buf_idx];

            let b64_idx = Self::postprocess_b64_inner(
                compressed_slice,
                b64_scratch_buffer,
                b64_scratch_buffer_len,
                &mut b64_out_buf,
            );

            b64_output.extend_from_slice(&b64_out_buf[..b64_idx]);

            if raw.is_empty() || cmp_buf_idx < compressed_buf.len() {
                return Ok(());
            }
        }
    }

    /// Encode some compressed bytes into the b64 output
    ///
    /// Returns the amount of bytes written to `b64_output_buffer`.
    ///
    /// # Limit
    /// This function has a limit to the input `compressed_slice`
    /// based on `N`. Ensure `N` is big enough for your needs.
    #[must_use]
    fn postprocess_b64_inner<const N: usize>(
        compressed_slice: &[u8],
        b64_scratch_buffer: &mut [u8; 2],
        b64_scratch_buffer_len: &mut u8,
        b64_output_buffer: &mut [u8; N],
    ) -> usize {
        assert!(
            base64::encoded_len(compressed_slice.len(), true).is_some_and(|enc_len| enc_len <= N),
            "N is too small or the given compressed slice is too large"
        );

        let total_len = compressed_slice.len() + usize::from(*b64_scratch_buffer_len);
        let processable_len = if total_len.is_multiple_of(3) {
            total_len
        } else {
            total_len.next_multiple_of(3) - 3
        };

        if processable_len == 0 {
            for byte in compressed_slice.iter().copied() {
                b64_scratch_buffer[*b64_scratch_buffer_len as usize] = byte;
                *b64_scratch_buffer_len += 1;
            }

            return 0;
        }

        // The first chunk may contain a mix of `b64_buffer` and `compressed_buf`
        let first_chunk: [u8; 3] = core::array::from_fn(|i| {
            #[expect(clippy::cast_possible_truncation, reason = "3 is below u8::MAX")]
            let i = i as u8;
            if let Some(compressed_idx) = i.checked_sub(*b64_scratch_buffer_len) {
                compressed_slice[compressed_idx as usize]
            } else {
                b64_scratch_buffer[i as usize]
            }
        });

        B64.encode_slice(first_chunk, &mut b64_output_buffer[..4])
            .unwrap();

        if processable_len == 3 {
            // That first chunk is all we could encode

            let unused_len = total_len - processable_len;

            debug_assert!(unused_len < 3);

            #[expect(
                clippy::cast_possible_truncation,
                reason = "unused_len is always less than 3"
            )]
            {
                *b64_scratch_buffer_len = unused_len as u8;
            }

            for (i, item) in b64_scratch_buffer.iter_mut().enumerate().take(unused_len) {
                let compressed_idx = compressed_slice.len() + i - unused_len;
                *item = compressed_slice[compressed_idx];
            }

            return 4;
        }

        // The rest may contain only `unprocessed`'s bytes
        let rest_length = processable_len - 3;
        let rest_start = usize::from(3 - *b64_scratch_buffer_len);
        let rest_end = rest_start + rest_length;
        let rest = &compressed_slice[rest_start..rest_end];

        debug_assert!(rest.len().is_multiple_of(3));
        debug_assert!(compressed_slice.len() >= rest_end);

        let bytes = B64.encode_slice(rest, &mut b64_output_buffer[4..]).unwrap() + 4;

        #[expect(
            clippy::cast_possible_truncation,
            reason = "total_len is at most 2 more than processable_len"
        )]
        {
            *b64_scratch_buffer_len = (total_len - processable_len) as u8;
        }

        for (idx, item) in b64_scratch_buffer
            .iter_mut()
            .take(*b64_scratch_buffer_len as usize)
            .enumerate()
        {
            *item = compressed_slice[idx + rest_end];
        }

        bytes
    }

    fn finish_into_vec(&mut self, output: &mut Vec<u8>) -> Result<(), TDEFLStatus> {
        match self {
            Self::Uncompressed => Ok(()),
            Self::Compressed { compressor } => {
                Self::finish_compression_into_vec(compressor, output)
            }
            Self::Base64 {
                compressor,
                b64_scratch_buffer,
                b64_scratch_buffer_len,
            } => Self::finish_base64_into_vec(
                compressor,
                b64_scratch_buffer,
                b64_scratch_buffer_len,
                output,
            ),
        }
    }

    /// Flush deflate buffers into the vec.
    fn finish_compression_into_vec(
        compressor: &mut CompressorOxide,
        output: &mut Vec<u8>,
    ) -> Result<(), TDEFLStatus> {
        let mut temp_buf = [0u8; Self::TEMP_BUFFER_SIZE];

        loop {
            let (status, _, out_idx) =
                compress(compressor, const { &[] }, &mut temp_buf, TDEFLFlush::Finish);

            output.extend_from_slice(&temp_buf[..out_idx]);

            match status {
                TDEFLStatus::Done => return Ok(()),
                TDEFLStatus::Okay => (),
                TDEFLStatus::BadParam | TDEFLStatus::PutBufFailed => return Err(status),
            }
        }
    }

    /// Flush deflate buffers and encode it as base64 into the vec.
    fn finish_base64_into_vec(
        compressor: &mut CompressorOxide,
        b64_scratch_buffer: &mut [u8; 2],
        b64_scratch_buffer_len: &mut u8,
        output: &mut Vec<u8>,
    ) -> Result<(), TDEFLStatus> {
        let mut compressed_buf = [0u8; Self::COMPRESSED_BUFFER_SIZE];
        let mut b64_out_buf = [0u8; Self::BASE64_BUFFER_SIZE];

        loop {
            let (status, _, out_idx) = compress(
                compressor,
                const { &[] },
                &mut compressed_buf,
                TDEFLFlush::Finish,
            );

            let compressed_slice = &compressed_buf[..out_idx];

            let b64_out_bytes = Self::postprocess_b64_inner(
                compressed_slice,
                b64_scratch_buffer,
                b64_scratch_buffer_len,
                &mut b64_out_buf,
            );

            output.extend_from_slice(&b64_out_buf[..b64_out_bytes]);

            match status {
                TDEFLStatus::Done => break,
                TDEFLStatus::Okay => (),
                TDEFLStatus::BadParam | TDEFLStatus::PutBufFailed => return Err(status),
            }
        }

        // Flush b64 buffers, if any
        let Some(b64_rem) = b64_scratch_buffer.get(..(*b64_scratch_buffer_len as usize)) else {
            return Ok(());
        };

        if b64_rem.is_empty() {
            return Ok(());
        }

        let mut rem_buf = [0u8; 4];

        let res = B64.encode_slice(b64_rem, &mut rem_buf);
        debug_assert_eq!(res, Ok(4));

        output.extend_from_slice(rem_buf.as_slice());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        slightly_random_data, ByteFeeder, SAMPLE_INPUT_DATA, SAMPLE_METADATA,
        SAMPLE_UNSORTED_INPUT_DATA, TEST_CHUNK_MAX_SIZE,
    };
    use fastrand::Rng;

    #[test]
    fn postprocess_compression() {
        const ROUNDS: usize = 1_000;

        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..ROUNDS {
            let data = slightly_random_data(&mut rng);
            let mut feeder = ByteFeeder::new(&data);

            let mut compressor = CompressorOxide::with_format_and_level(
                DataFormat::Zlib,
                CompressionLevel::BestSpeed,
            );

            let mut compressed = Vec::with_capacity(data.len());

            while !feeder.is_empty() {
                ReplayEncoderPostprocessor::postprocess_compression(
                    &mut compressor,
                    feeder.bite(&mut rng),
                    &mut compressed,
                )
                .expect("compression should work");
            }

            ReplayEncoderPostprocessor::finish_compression_into_vec(
                &mut compressor,
                &mut compressed,
            )
            .expect("compression should finish");

            let decompressed = miniz_oxide::inflate::decompress_to_vec_zlib(&compressed)
                .expect("decompression should work");

            assert_eq!(decompressed.as_slice(), data.as_slice());
        }
    }

    #[test]
    fn postprocess_base64() {
        const ROUNDS: usize = 1_000;

        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..ROUNDS {
            let data = slightly_random_data(&mut rng);

            let mut feeder = ByteFeeder::new(&data);

            let mut compressor = CompressorOxide::with_format_and_level(
                DataFormat::Zlib,
                CompressionLevel::BestSpeed,
            );

            let mut b64_scratch_buffer = [0u8; 2];
            let mut b64_scratch_buffer_len = 0;

            let mut b64_output = Vec::with_capacity(data.len());

            while !feeder.is_empty() {
                ReplayEncoderPostprocessor::postprocess_b64(
                    &mut compressor,
                    &mut b64_scratch_buffer,
                    &mut b64_scratch_buffer_len,
                    feeder.bite(&mut rng),
                    &mut b64_output,
                )
                .expect("compression and encode should work");
            }

            ReplayEncoderPostprocessor::finish_base64_into_vec(
                &mut compressor,
                &mut b64_scratch_buffer,
                &mut b64_scratch_buffer_len,
                &mut b64_output,
            )
            .expect("compression and encode should finish");

            let decoded = B64.decode(&b64_output).expect("decode should work");
            let decompressed = miniz_oxide::inflate::decompress_to_vec_zlib(&decoded)
                .expect("decompression should work");

            assert_eq!(decompressed.as_slice(), data.as_slice());
        }
    }

    #[test]
    fn postprocess_base64_inner() {
        const ROUNDS: usize = 1_000;

        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..ROUNDS {
            let data = slightly_random_data(&mut rng);

            let mut feeder = ByteFeeder::new(data.as_slice());

            let mut b64_scratch_buffer = [0u8; 2];
            let mut b64_scratch_buffer_len = 0u8;
            // let mut b64_output_buffer = vec![0u8; base64::encoded_len(data.len(), true).unwrap()];
            let mut b64_output_buffer =
                [0u8; base64::encoded_len(TEST_CHUNK_MAX_SIZE, true).unwrap()];
            let mut encoded = Vec::new();

            while !feeder.is_empty() {
                let out_len = ReplayEncoderPostprocessor::postprocess_b64_inner(
                    feeder.bite(&mut rng),
                    &mut b64_scratch_buffer,
                    &mut b64_scratch_buffer_len,
                    &mut b64_output_buffer,
                );

                encoded.extend_from_slice(&b64_output_buffer[..out_len]);
            }

            if b64_scratch_buffer_len > 0 {
                encoded.extend_from_slice(
                    B64.encode(&b64_scratch_buffer[..b64_scratch_buffer_len as usize])
                        .as_bytes(),
                );
            }

            let decoded = B64
                .decode(encoded.as_slice())
                .expect("decoding should work");

            assert_eq!(data.as_slice(), decoded.as_slice());
        }
    }

    #[test]
    fn metadata_encoder_state() {
        let mut encoder = ReplayEncoderState::WaitingForMetadata;

        assert!(matches!(
            encoder
                .feed_input_data(&[], &mut [])
                .expect_err("this should error"),
            ReplaySerializeError::InvalidOperation,
        ));

        encoder
            .feed_metadata(&SAMPLE_METADATA, None)
            .expect("this should work");
    }

    #[test]
    fn input_encoder_state() {
        let mut encoder = ReplayEncoderState::InputData {
            prev_frame: 0,
            parse_mode: InputParseMode::Relative,
        };

        assert!(matches!(
            encoder
                .feed_metadata(&SAMPLE_METADATA, None)
                .expect_err("this should error"),
            ReplaySerializeError::InvalidOperation,
        ));

        let mut output_buffer = [0u8; 16];

        let tuple = encoder
            .feed_input_data(SAMPLE_INPUT_DATA.as_slice(), &mut output_buffer)
            .expect("serialization should work");

        assert_eq!(tuple, (2, 4));

        let mut encoder = ReplayEncoderState::InputData {
            prev_frame: 0,
            parse_mode: InputParseMode::Relative,
        };

        assert!(matches!(
            encoder
                .feed_input_data(SAMPLE_UNSORTED_INPUT_DATA.as_slice(), &mut output_buffer)
                .expect_err("this should not work"),
            ReplaySerializeError::UnsortedInput {
                prev_time: 9,
                unsorted_time: 3
            }
        ));
    }

    #[test]
    fn b64_postprocessor_returns_b64_string() {
        const ROUNDS: usize = 1_000;

        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..ROUNDS {
            let mut postprocessor = ReplayEncoderPostprocessor::new(ReplayBufferKind::Base64, 0);

            let data = slightly_random_data(&mut rng);
            let mut out = Vec::with_capacity(data.len());

            postprocessor
                .postprocess_into_vec(data.as_slice(), &mut out)
                .expect("postprocessing should work");
            postprocessor
                .finish_into_vec(&mut out)
                .expect("postprocessor should finish");

            let string =
                AsciiString::from_ascii(out).expect("postprocessor output should be valid ascii");

            B64.decode(string.as_bytes())
                .expect("postprocessor output should be valid base64");
        }
    }
}
