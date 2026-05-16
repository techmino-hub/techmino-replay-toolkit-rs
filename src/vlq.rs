use core::{
    iter::{Copied, FusedIterator},
    num::NonZeroU8,
};

/// A struct representing VLQ-encoded data.
///
/// Provides the bridge between raw `u64` values and
/// VLQ-encoded `[u8]`s.
///
/// The maximum size of the `[u8]` array is 8 bytes long,
/// which means it has a limit of representing up to
/// 56 bits (since one bit of each byte is used by the format).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VlqData {
    /// Length ranges from 1 to 8
    len: NonZeroU8,

    /// The byte array representing the VLQ-encoded data.
    ///
    /// Only the `..len` range is valid VLQ data.
    bytes: [u8; Self::MAX_BYTES as usize],
}

impl VlqData {
    /// The maximum representable u64 in the vlq format.
    pub const MAX_REPRESENTABLE: u64 = (1 << 56) - 1;
    /// The maximum amount of output vlq bytes per value.
    pub const MAX_BYTES: u8 = 8;

    /// Gets the length, in bytes, of the VLQ representation of the
    /// given u64.
    ///
    /// A [`None`] is returned when `value` exceeds [`Self::MAX_REPRESENTABLE`]
    /// and cannot be represented as a VLQ.
    #[must_use]
    pub const fn value_repr_len(value: u64) -> Option<NonZeroU8> {
        if value > Self::MAX_REPRESENTABLE {
            return None;
        }

        let Some(ilog2) = value.checked_ilog2() else {
            return Some(NonZeroU8::MIN);
        };

        #[expect(
            clippy::cast_possible_truncation,
            reason = "ilog2 can only return up to 63"
        )]
        let ilog2 = ilog2 as u8;

        NonZeroU8::new(ilog2 / 7 + 1)
    }

    /// Converts a u64 value into a VLQ-encoded value.
    ///
    /// An `Err` is returned when `value` exceeds [`Self::MAX_REPRESENTABLE`]
    /// and cannot be represented as a VLQ.
    const fn new(mut value: u64) -> Result<Self, VlqEncodeError> {
        let Some(len) = Self::value_repr_len(value) else {
            return Err(VlqEncodeError { number: value });
        };
        let mut idx = len.get() - 1;

        let mut bytes = [0u8; Self::MAX_BYTES as usize];

        bytes[idx as usize] = (value & 0x7F) as u8;
        value >>= 7;

        #[expect(
            clippy::cast_possible_truncation,
            reason = "Truncation here is completely expected"
        )]
        while value > 0 {
            idx -= 1;
            bytes[idx as usize] = (value | 0x80) as u8;
            value >>= 7;
        }

        Ok(Self { len, bytes })
    }

    #[must_use]
    const fn value(&self) -> u64 {
        let mut value = 0u64;

        let mut idx = 0;

        while idx < self.len.get() {
            let byte = self.bytes[idx as usize];

            value <<= 7;
            value |= (byte & 0x7F) as u64;

            idx += 1;
        }

        value
    }

    /// Get an iterator into the VLQ-encoded bytes.
    fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        self.as_slice().iter().copied()
    }

    /// Get the VLQ-encoded bytes as a slice.
    #[must_use]
    fn as_slice(&self) -> &[u8] {
        self.bytes.get(..self.len.get() as usize).unwrap()
    }

    /// Get an iterator of VLQ-encoded bytes.
    fn multi_iter(vlqs: &[Self]) -> impl Iterator<Item = u8> + '_ {
        vlqs.iter().flat_map(VlqData::iter)
    }

    /// Get a [`Vec`] of VLQ-encoded bytes.
    #[must_use]
    fn multi_to_vec(vlqs: &[Self]) -> Vec<u8> {
        VlqData::multi_iter(vlqs).collect()
    }
}

impl TryFrom<u64> for VlqData {
    type Error = VlqEncodeError;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        VlqData::new(value)
    }
}

impl<'v> From<&'v VlqData> for &'v [u8] {
    fn from(value: &'v VlqData) -> Self {
        value.as_slice()
    }
}

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

impl<'v> VlqReader<Copied<core::slice::Iter<'v, u8>>> {
    /// Creates a new `VlqReader` from a slice.
    #[must_use]
    pub(crate) fn from_slice(slice: &'v [u8]) -> Self {
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
            let byte = self.0.next()?;
            buf[idx as usize] = byte;
            idx += 1;
            if byte.cast_signed() >= 0 {
                break;
            }
        }

        if idx as usize == buf.len() && buf.last().unwrap().cast_signed() < 0 {
            return Some(Err(VlqDecodeError { partial_vlq: buf }));
        }

        let data = VlqData {
            bytes: buf,
            len: NonZeroU8::new(idx).unwrap(),
        };

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

/// There was an attempt to encode an oversized `u64` into the VLQ format.
#[derive(Debug)]
pub struct VlqEncodeError {
    /// The `u64` value that couldn't be encoded into the VLQ format.
    number: u64,
}

/// There was an attempt to decode an oversized VLQ into a `u64`.
#[derive(Debug)]
pub struct VlqDecodeError {
    /// Part of the VLQ byte array that couldn't be encoded into the `u64` format.
    partial_vlq: [u8; VlqData::MAX_BYTES as usize],
}

#[cfg(test)]
mod tests {
    use fastrand::Rng;

    use crate::{GameReplayData, InputParseMode};

    use super::*;

    fn create_vlqs(values: &[u64]) -> Vec<u8> {
        // Estimation: most values need around 2 bytes
        let mut vlqs = Vec::with_capacity(values.len() * 2);

        // u64 is up to 9 VLQ bytes
        let mut vlq = Vec::with_capacity(9);
        for &value in values {
            vlq.clear();
            let mut value = value;

            vlq.push((value & 0x7F) as u8);
            value >>= 7;

            while value > 0 {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "This is already masked and should never truncate"
                )]
                vlq.push(((value & 0x7F) | 0x80) as u8);
                value >>= 7;
            }

            vlq.reverse();
            vlqs.append(&mut vlq);
        }

        vlqs
    }

    #[test]
    fn test_vlq_creation() {
        // Mostly sourced from https://en.wikipedia.org/wiki/Variable-length_quantity#Examples
        let cases = [
            (vec![0x00], 0x00),
            (vec![0x01], 0x01),
            (vec![0x7F], 0x7F),
            (vec![0x81, 0x00], 0x80),
            (vec![0xC0, 0x00], 0x2000),
            (vec![0xFF, 0x7F], 0x3FFF),
            (vec![0x81, 0x80, 0x00], 0x4000),
            (vec![0xFF, 0xFF, 0x7F], 0x001F_FFFF),
            (vec![0xFF, 0xFF, 0x7F], 0x001F_FFFF),
            (vec![0x81, 0x80, 0x80, 0x00], 0x0020_0000),
            (vec![0x01], 1),
            (vec![0x8F, 0x00], 1920),
        ];

        for (expected_vlqs, value) in cases {
            let vlq = VlqData::new(value).unwrap();
            assert_eq!(vlq.as_slice(), expected_vlqs.as_slice());
        }
    }

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

    #[test]
    fn test_vlq_roundtrip() {
        for i in 0..u64::from(u16::MAX) {
            let vlq = VlqData::new(i).unwrap();

            assert_eq!(vlq.value(), i);
        }

        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..1_000_000 {
            let val = rng.u64(0..=VlqData::MAX_REPRESENTABLE);
            let vlq = VlqData::new(val).unwrap();
            assert_eq!(vlq.value(), val);
        }

        assert_eq!(
            VlqData::new(VlqData::MAX_REPRESENTABLE).unwrap().value(),
            VlqData::MAX_REPRESENTABLE
        );
    }
}
