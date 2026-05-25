use std::{borrow::Cow, os::unix::process};

use alloc::string::String;
use alloc::vec::Vec;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use miniz_oxide::{
    inflate::{self, core::DecompressorOxide, stream::InflateState, TINFLStatus},
    MZError,
};

use crate::{
    format::ReplayBufferKind,
    types::{GameInputEvent, GameReplayData, GameReplayMetadata, InputParseMode, ReplayParseError},
    vlq::{VlqData, VlqDecodeError, VlqReader, VlqReaderSM},
    InputAction,
};

impl GameReplayData {
    /// Parses a base64 string into a game replay.
    ///
    /// For parsing a replay from the contents of a `.rep` file in the game's `replays` directory,
    /// see [`parse_compressed_bytes`] instead.
    ///
    /// `parse_mode` is an optional argument used to specify how you want the inputs to be parsed.\
    /// This is useful for preventing errors from occurring if this function fails to recognize
    /// the game version to automatically infer its parse mode.\
    /// For more information, see [`InputParseMode`].
    ///
    /// # Errors
    /// For more information on possible errors, look at the [`ReplayParseError`] struct.
    pub fn try_from_base64(
        string: &str,
        parse_mode: Option<InputParseMode>,
    ) -> Result<GameReplayData, ReplayParseError> {
        let data = B64.decode(string)?;

        Self::try_from_compressed(&data, parse_mode)
    }

    /// Parses a compressed byte array into a game replay.
    ///
    /// The byte array can be in the form of the contents of a `.rep` file in the game's `replays` directory.
    ///
    /// For parsing a replay from a base64 string, see [`parse_base64`] instead.
    ///
    /// `parse_mode` is an optional argument used to specify how you want the inputs to be parsed.\
    /// This is useful for preventing errors from occurring if this function fails to recognize
    /// the game version to automatically infer its parse mode.
    /// For more information, see [`InputParseMode`].
    ///
    /// # Errors
    /// For more information on possible errors, look at the [`ReplayParseError`] struct.
    pub fn try_from_compressed(
        data: &[u8],
        parse_mode: Option<InputParseMode>,
    ) -> Result<GameReplayData, ReplayParseError> {
        let data = inflate::decompress_to_vec_zlib(data)?;

        Self::try_from_raw(&data, parse_mode)
    }

    /// Parses a raw, uncompressed byte array into a game replay.
    ///
    /// Usually, Techmino compresses the replay using `zlib` before saving it, either as a
    /// base64 string, or a `.rep` file in the game's `replays` directory.\
    /// In which case, this is not what you are looking for.\
    /// See [`parse_base64`] and [`parse_compressed_bytes`] instead.
    ///
    /// This function is only useful if you managed to get the replay in the uncompressed form,
    /// which doesn't usually seem to be the case.
    ///
    /// # Errors
    /// For more information on possible errors, look at the [`ReplayParseError`] struct.
    pub fn try_from_raw(
        data: &[u8],
        parse_mode: Option<InputParseMode>,
    ) -> Result<GameReplayData, ReplayParseError> {
        let Some(first_newline) = data.iter().position(|&el| el == b'\n') else {
            return Err(ReplayParseError::MetadataSeparatorNotFound);
        };

        let (metadata_slice, input_slice) = data.split_at(first_newline);

        // This will never panic becaause we already know that there is a
        // separator
        let input_slice = &input_slice[1..];

        let metadata = GameReplayMetadata::try_from(metadata_slice)?;

        let Some(parse_mode) =
            parse_mode.or_else(|| InputParseMode::try_infer_from_version(&metadata.version))
        else {
            return Err(ReplayParseError::UnknownInputParseMode(metadata.version));
        };

        Ok(GameReplayData {
            inputs: parse_input_slice(input_slice, parse_mode)?,
            metadata,
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

pub(crate) fn parse_input_slice(
    input_slice: &[u8],
    parse_mode: InputParseMode,
) -> Result<Vec<GameInputEvent>, ReplayParseError> {
    todo!();
    // let mut events = Vec::with_capacity(input_slice.len());

    // let mut prev_timestamp = 0;
    // for (position, chunk) in values.chunks_exact(2).enumerate() {
    //     let (time, key) = (chunk[0], chunk[1]);

    //     let frame = match parse_mode {
    //         InputParseMode::Relative => time + prev_timestamp,
    //         InputParseMode::Absolute => time,
    //     };

    //     let Ok(key) = u8::try_from(key) else {
    //         return Err(ReplayParseError::MalformedInputData {
    //             position: position as u64 * 2,
    //             frame,
    //             kind: key,
    //         });
    //     };

    //     let kind = InputEventKind::from(key > 0b0010_0000);
    //     let Ok(key) = InputEventKey::try_from(key & 0b0001_1111) else {
    //         return Err(ReplayParseError::MalformedInputData {
    //             frame,
    //             position: position as u64 * 2,
    //             kind: u64::from(key),
    //         });
    //     };

    //     let Ok(event) = GameInputEvent::new(kind, key, frame) else {
    //         return Err(ReplayParseError::MalformedInputData {
    //             frame,
    //             position: position as u64 * 2,
    //             kind: u64::from(u8::from(key)),
    //         });
    //     };

    //     prev_timestamp = frame;

    //     events.push(event);
    // }

    // Ok(events)
}

/// Gets the raw frame-key input pairs, with no relative/absolute
/// processing.
///
/// Use [`parse_input_iter`] for the version with that kind of processing.
fn get_raw_input_pairs<I>(byte_iter: I) -> RawInputPairsIter<I>
where
    I: Iterator<Item = u8>,
{
    RawInputPairsIter(Some(byte_iter))
}

struct RawInputPairsIter<I: Iterator<Item = u8>>(Option<I>);

impl<I> Iterator for RawInputPairsIter<I>
where
    I: Iterator<Item = u8>,
{
    type Item = Result<(VlqData, InputAction), InvalidInputDataError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut reader = VlqReader::new(self.0.take()?);

        let frame = match reader.next()? {
            Ok(v) => v,
            Err(e) => return Some(Err(InvalidInputDataError::VlqDecodeError(e))),
        };

        let mut iter = reader.into_inner();

        let actioncode = iter.next()?;

        let action = match InputAction::try_from(actioncode) {
            Ok(a) => a,
            Err(e) => return Some(Err(InvalidInputDataError::InvalidAction(e))),
        };

        self.0.replace(iter);

        Some(Ok((frame, action)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let Some(ref iter) = self.0 else {
            return (0, Some(0));
        };

        let bytes_hint = iter.size_hint();

        // Min case: Each frame is 8 bytes, so each event is 9 bytes
        let min = bytes_hint.0 / 9;

        // Max case: Each frame is 1 byte, so each event is 2 bytes
        let max = bytes_hint.1.map(|max| max / 2);

        (min, max)
    }
}

#[derive(Debug)]
pub enum InvalidInputDataError {
    VlqDecodeError(VlqDecodeError),
    InvalidAction(<InputAction as TryFrom<u8>>::Error),
}

impl From<VlqDecodeError> for InvalidInputDataError {
    fn from(value: VlqDecodeError) -> Self {
        Self::VlqDecodeError(value)
    }
}
impl From<<InputAction as TryFrom<u8>>::Error> for InvalidInputDataError {
    fn from(value: <InputAction as TryFrom<u8>>::Error) -> Self {
        Self::InvalidAction(value)
    }
}

/// A decoder for Techmino replays.
pub struct ReplayDecoder {
    state: ReplayDecoderState,
    preprocessor: ReplayDecoderPreprocessor,
}

impl ReplayDecoder {
    /// Create a new [`ReplayDecoder`].
    ///
    /// You must give the kind of replay data you will feed this decoder.
    /// For more information, see [`ReplayBufferKind`].
    pub fn new(kind: ReplayBufferKind) -> Self {
        Self {
            state: ReplayDecoderState::WaitingForMetadata {
                buf: Vec::with_capacity(4096),
            },
            preprocessor: ReplayDecoderPreprocessor::new(kind),
        }
    }

    /// Feed this [`ReplayDecoder`] some data.
    ///
    /// This can be used multiple times. For example, if you have
    /// a stream of bytes, you can gradually feed the bytes into this
    /// update function.
    pub fn update(&mut self, bytes: &[u8]) -> Result<Decoded, ReplayParseError> {
        let uncompressed = match self.preprocessor.preprocess(bytes) {
            Ok(b) => b,
            Err(FormatError::Base64Error(e)) => return Err(ReplayParseError::Base64DecodeError(e)),
            Err(FormatError::ZlibError { status, mz_error }) => {
                return Err(ReplayParseError::ZlibDecompressError {
                    status,
                    mz_error: Some(mz_error),
                })
            }
        };

        self.state.update(bytes)
    }
}

/// The state machine for the replay decoder.
enum ReplayDecoderState {
    /// Waiting for the metadata section to finish.
    WaitingForMetadata {
        /// The current cumulative decompressed buffer while waiting for the
        /// newline (`\n` = `0xA`) character to get produced.
        buf: Vec<u8>,
    },
    /// Decoding inputs.
    InputDecode { vlq_reader: VlqReaderSM },
}

impl ReplayDecoderState {
    /// Run an iteration of the decoder.
    ///
    /// **`bytes` is expected to be in uncompressed/raw format.**
    fn update(&mut self, bytes: &[u8]) -> Result<Decoded, ReplayParseError> {
        match self {
            Self::WaitingForMetadata { buf } => {
                let prev_buf_len = buf.len();
                buf.extend_from_slice(bytes);

                if let Some(newline_pos_in_input) = bytes.iter().position(|b| *b == b'\n') {
                    let newline_pos_in_buf = newline_pos_in_input + prev_buf_len;

                    let metadata = Self::finish_metadata(&buf[..newline_pos_in_buf])?;

                    let vlq_bytes = buf.get(newline_pos_in_buf + 1..);
                    let mut vlq_reader = VlqReaderSM::new();
                    let mut inputs = Vec::new();

                    if let Some(vb) = vlq_bytes {
                        inputs.push(
                            GameInputEvent::new(
                                0,
                                InputAction {
                                    kind: crate::InputActionKind::Press,
                                    key: crate::InputActionKey::Down1,
                                },
                            )
                            .unwrap(),
                        );
                        todo!("feed bytes to vlq reader");
                    }

                    *self = Self::InputDecode { vlq_reader };

                    return Ok(Decoded {
                        metadata: Some(Box::new(metadata)),
                        inputs,
                    });
                }

                Ok(Decoded {
                    metadata: None,
                    inputs: Vec::new(),
                })
            }
            Self::InputDecode { vlq_reader } => {
                todo!();
            }
        }
    }

    /// Called when the end of metadata has been found.
    ///
    /// **`bytes` is expected to be in uncompressed/raw format.**
    fn finish_metadata(metadata: &[u8]) -> Result<GameReplayMetadata, ReplayParseError> {
        Ok(serde_json::from_slice::<GameReplayMetadata>(metadata)?)
    }

    fn decode_inputs(
        vlq_reader: &mut VlqReaderSM,
    ) -> Result<Vec<GameInputEvent>, ReplayParseError> {
        todo!();
    }
}

/// Preprocesses different replay data forms into uncompressed form.
enum ReplayDecoderPreprocessor {
    /// The base64 preprocessor.
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

        if let Err(e) = engine.decode_vec(rest, &mut compressed) {
            return Err(FormatError::Base64Error(e));
        }

        debug_assert!(unprocessed.len() >= rest_end);

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

/// Return the data was decoded in the last update call.
///
/// Not cumulative; if you want a full replay, you have to build it yourself.
#[must_use = "the newly-decoded data is in the `Decoded` struct"]
pub struct Decoded {
    /// Whether or not the metadata finished getting decoded in this update call.
    ///
    /// The metadata will only ever be returned once!
    metadata: Option<Box<GameReplayMetadata>>,

    /// The inputs decoded in this update call.
    inputs: Vec<GameInputEvent>,
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use fastrand::Rng;

    use crate::{deserialize::ReplayDecoderPreprocessor, format::ReplayBufferKind};

    const PREPROCESSOR_TRIALS: usize = 1024;

    const TEST_DATA_UNCOMPRESSED_LEN: usize = 16384;

    /// At max, how many bits in a byte to turn on.
    const TEST_DATA_BIT_PER_BYTE: usize = 2;

    const TEST_CHUNK_MAX_SIZE: usize = 48;

    /// Creates not-quite-random data.
    fn create_data(rng: &mut Rng) -> [u8; TEST_DATA_UNCOMPRESSED_LEN] {
        core::array::from_fn::<u8, TEST_DATA_UNCOMPRESSED_LEN, _>(|_| {
            // For every byte, choose at most 3 random bits to turn on
            let mut byte = 0;

            for _ in 0..TEST_DATA_BIT_PER_BYTE {
                let bit = rng.u8(..8);

                let mask = 1u8 << bit;

                byte |= mask;
            }

            byte
        })
    }

    /// A struct to split an input data into randomly-sized chunks.
    struct ByteFeeder<'a> {
        /// The slice representing the yet-to-be-output data.
        data: &'a [u8],
    }

    impl<'a> ByteFeeder<'a> {
        /// Creates a new byte feeder.
        fn new(data: &'a [u8]) -> Self {
            Self { data }
        }

        /// Get a randomly-sized chunk of data.
        fn bite(&mut self, rng: &mut Rng) -> &'a [u8] {
            let chunk_size = rng.usize(1..=(TEST_CHUNK_MAX_SIZE.min(self.data.len())));
            let (chunk, rest) = self.data.split_at(chunk_size);
            self.data = rest;
            chunk
        }

        /// Whether or not this is empty.
        fn is_empty(&self) -> bool {
            self.data.is_empty()
        }
    }

    #[test]
    fn internal_test_byte_feeder() {
        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..1024 {
            let init_data = create_data(&mut rng);
            let mut feeder = ByteFeeder::new(init_data.as_slice());
            let mut feeder_output = Vec::with_capacity(TEST_DATA_UNCOMPRESSED_LEN);

            while !feeder.is_empty() {
                feeder_output.extend_from_slice(feeder.bite(&mut rng));
            }

            assert_eq!(init_data.as_slice(), feeder_output.as_slice());
        }
    }

    #[test]
    fn preprocess_uncompressed() {
        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..PREPROCESSOR_TRIALS {
            let init_data = create_data(&mut rng);
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
            let init_data = create_data(&mut rng);

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
            let init_data = create_data(&mut rng);

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
            let init_data = create_data(&mut rng);

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
}
