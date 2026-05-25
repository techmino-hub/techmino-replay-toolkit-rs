use core::{
    iter::{Copied, FusedIterator},
    num::NonZeroU8,
};

use crate::vlq::VlqData;

/// From a byte iterator, read it and get some [`VlqData`] instances.
pub(crate) struct VlqReader<I: Iterator<Item = u8>>(I);

impl<I: Iterator<Item = u8>> VlqReader<I> {
    /// Creates a new `VlqReader` which reads from an iterator.
    #[must_use]
    pub(crate) fn new(iterator: I) -> Self {
        Self(iterator)
    }

    /// Gives back the iterator at the current state.
    pub(crate) fn into_inner(self) -> I {
        self.0
    }
}

impl<'a> VlqReader<Copied<core::slice::Iter<'a, u8>>> {
    /// Creates a new `VlqReader` from a slice.
    #[must_use]
    pub(crate) fn from_slice(slice: &'a [u8]) -> Self {
        Self(slice.iter().copied())
    }
}

impl<I> Iterator for VlqReader<I>
where
    I: Iterator<Item = u8>,
{
    type Item = Result<VlqData, VlqDecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Find end of vlq value
        // .next() until we get one where msb is OFF
        let mut buf = [0u8; VlqData::MAX_BYTES as usize];

        let mut idx = 0u8;

        while (idx as usize) < buf.len() {
            let byte = match self.0.next() {
                Some(b) => b,
                None => {
                    if idx == 0 {
                        return None;
                    } else {
                        return Some(Err(VlqDecodeError::UnexpectedEof {}));
                    }
                }
            };

            buf[idx as usize] = byte;
            idx += 1;
            if byte.cast_signed() >= 0 {
                break;
            }
        }

        if idx as usize == buf.len() && buf.last().unwrap().cast_signed() < 0 {
            return Some(Err(VlqDecodeError::OversizedVlq { partial_vlq: buf }));
        }

        let data = VlqData::from_raw_parts(NonZeroU8::new(idx).unwrap(), buf);

        Some(Ok(data))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Minimum case: A single really large vlq value
        let min = self.0.size_hint().0 / VlqData::MAX_BYTES as usize;
        // Maximum case: A lot of single-byte "vlq"s
        let max = self.0.size_hint().1;

        (min, max)
    }
}

impl<I> FusedIterator for VlqReader<I> where I: Iterator<Item = u8> + FusedIterator {}

/// A VLQ reader state machine that can take in arbitrary chunks
/// of VLQ-encoded bytes.
pub(crate) struct VlqReaderSM {
    buf: [u8; (VlqData::MAX_BYTES - 1) as usize],
    buf_len: u8,
}

impl VlqReaderSM {
    pub(crate) fn new() -> Self {
        Self {
            buf: [0u8; _],
            buf_len: 0,
        }
    }
}

/// Something went wrong trying to decode a VLQ.
#[derive(Debug)]
pub enum VlqDecodeError {
    /// There was an attempt to decode an oversized VLQ into a `u64`.
    OversizedVlq {
        /// Part of the VLQ byte array that couldn't be encoded into the `u64` format.
        partial_vlq: [u8; VlqData::MAX_BYTES as usize],
    },
    /// The iterator finished unexpectedly.
    UnexpectedEof {},
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vlq_extraction() {
        // Mostly sourced from https://en.wikipedia.org/wiki/Variable-length_quantity#Examples
        let cases = [
            (vec![0x00], vec![0x00]),
            (vec![0x01], vec![0x01]),
            (vec![0x7F], vec![0x7F]),
            (vec![0x81, 0x00], vec![0x80]),
            (vec![0xC0, 0x00], vec![0x2000]),
            (vec![0xFF, 0x7F], vec![0x3FFF]),
            (vec![0x81, 0x80, 0x00], vec![0x4000]),
            (vec![0xFF, 0xFF, 0x7F], vec![0x001F_FFFF]),
            (
                vec![0xFF, 0xFF, 0x7F, 0xFF, 0xFF, 0x7F],
                vec![0x001F_FFFF, 0x001F_FFFF],
            ),
            (vec![0x81, 0x80, 0x80, 0x00], vec![0x0020_0000]),
            (vec![0x01, 0x01, 0x01], vec![1, 1, 1]),
            (vec![0x8F, 0x00], vec![1920]),
        ];

        for (input, expected) in cases {
            let values: Box<[u64]> = VlqReader::from_slice(input.as_slice())
                .map(Result::unwrap)
                .map(|x| x.value())
                .collect();

            assert_eq!(&*values, expected.as_slice());
        }
    }
}
