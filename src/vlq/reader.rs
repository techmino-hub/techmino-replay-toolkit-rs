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
                    }
                    return Some(Err(VlqDecodeError::UnexpectedEof {}));
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

        let data = VlqData::from_raw(buf);

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
    /// Create a new instance of a VLQ reader state machine.
    pub(crate) const fn new() -> Self {
        Self {
            buf: [0u8; _],
            buf_len: 0,
        }
    }

    /// Feed a VLQ-encoded chunk into this state machine and
    /// append the VLQ data points into an existing Vec.
    pub(crate) fn update_to_vec(
        &mut self,
        vlq_encoded: &[u8],
        vlq_data_points: &mut Vec<VlqData>,
    ) -> Result<(), VlqDecodeError> {
        let mut buf = [
            self.buf[0],
            self.buf[1],
            self.buf[2],
            self.buf[3],
            self.buf[4],
            self.buf[5],
            self.buf[6],
            0,
        ];

        for input in vlq_encoded.iter().copied() {
            if self.buf_len as usize == buf.len() {
                self.buf = buf[..(VlqData::MAX_BYTES - 1) as usize].try_into().unwrap();
                return Err(VlqDecodeError::OversizedVlq { partial_vlq: buf });
            }

            buf[self.buf_len as usize] = input;
            self.buf_len += 1;

            if input < 0x80 {
                let data = VlqData::from_raw(buf);
                vlq_data_points.push(data);
                self.buf_len = 0;
            }
        }

        Ok(())
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
    // TODO: Get rid of this after finishing migration to SM
    /// The iterator finished unexpectedly.
    UnexpectedEof {},
}

#[cfg(test)]
mod tests {
    use fastrand::Rng;

    use crate::tests::{random_vlq, ByteFeeder};

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

    #[test]
    fn test_vlq_extraction_sm() {
        // TODO: Increase to a million
        const VLQ_AMOUNT: usize = 10;

        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);
        let input_vlqs: Box<[_]> = (0..VLQ_AMOUNT).map(|_| random_vlq(&mut rng)).collect();
        let data: Box<[_]> = input_vlqs.iter().flat_map(VlqData::iter).collect();

        let mut feeder = ByteFeeder::new(&data);

        let mut sm = VlqReaderSM::new();
        let mut output_vlqs = Vec::with_capacity(input_vlqs.len());

        while !feeder.is_empty() {
            sm.update_to_vec(&data, &mut output_vlqs)
                .expect("sm shouldn't error");

            while !feeder.is_empty() {
                let _ = feeder.bite(&mut rng);
                println!("{}", feeder.len());
            }
            // TODO: re-enable
            // sm.update_to_vec(feeder.bite(&mut rng), &mut output_vlqs)
            //     .expect("sm shouldn't error");
        }

        assert_eq!(*input_vlqs, *output_vlqs);
        todo!("passed temp full buffer version, retry with feeding");
    }
}
