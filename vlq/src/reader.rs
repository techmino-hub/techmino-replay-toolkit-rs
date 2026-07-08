use core::fmt::{self, Display};

use crate::VlqData;
use alloc::vec::Vec;
use thiserror::Error;

/// A VLQ reader state machine that can take in arbitrary chunks
/// of VLQ-encoded bytes.
#[derive(Clone, Debug)]
pub struct VlqReader {
    buf: [u8; Self::BUF_SIZE],
    buf_len: u8,
}

impl VlqReader {
    const BUF_SIZE: usize = (VlqData::MAX_BYTES - 1) as usize;

    /// Create a new instance of a VLQ reader state machine.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buf: [0u8; Self::BUF_SIZE],
            buf_len: 0,
        }
    }

    /// Feed a VLQ-encoded chunk into this state machine and
    /// append the VLQ data points into an existing Vec.
    ///
    /// # Errors
    /// This function errors when a certain value in the input VLQ array exceeds
    /// the limit for individual VLQ values.
    pub fn update_to_vec(
        &mut self,
        vlq_encoded: &[u8],
        vlq_data_points: &mut Vec<VlqData>,
    ) -> Result<(), VlqDecodeError> {
        let mut buf: [u8; Self::BUF_SIZE + 1] = [
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
                debug_assert!(buf.len() >= Self::BUF_SIZE);
                // SAFETY: We already sliced it to be the size of Self::BUF_SIZE,
                // so it should already be the correct size
                let arr: [u8; Self::BUF_SIZE] =
                    unsafe { buf[..Self::BUF_SIZE].try_into().unwrap_unchecked() };
                self.buf = arr;
                return Err(VlqDecodeError { partial_vlq: buf });
            }

            buf[self.buf_len as usize] = input;
            self.buf_len += 1;

            if input < 0x80 {
                let data = VlqData::from_raw(buf);
                vlq_data_points.push(data);
                self.buf_len = 0;
            }
        }

        debug_assert!(buf.len() >= Self::BUF_SIZE);
        // SAFETY: We already sliced it to be the size of Self::BUF_SIZE,
        // so it should already be the correct size
        let arr: [u8; Self::BUF_SIZE] =
            unsafe { buf[..Self::BUF_SIZE].try_into().unwrap_unchecked() };
        self.buf = arr;

        Ok(())
    }

    /// Returns false if this struct has any leftover partial data.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.buf_len == 0
    }
}

impl Default for VlqReader {
    fn default() -> Self {
        Self::new()
    }
}

/// There was an attempt to decode an oversized VLQ into a `u64`.
#[derive(Debug, Error)]
pub struct VlqDecodeError {
    /// Part of the VLQ byte array that couldn't be encoded into the `u64` format.
    pub partial_vlq: [u8; VlqData::MAX_BYTES as usize],
}

impl Display for VlqDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VLQ slice cannot fit into u64: {:?}", self.partial_vlq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{random_vlq, ByteFeeder};
    use alloc::boxed::Box;
    use fastrand::Rng;

    /// Get an iterator of VLQ-encoded bytes from a slice of [`VlqData`] instances.
    fn vlq_multi_iter(vlqs: &[VlqData]) -> impl Iterator<Item = u8> + '_ {
        vlqs.iter().flat_map(|v| v.as_slice().iter().copied())
    }

    #[test]
    fn test_vlq_extraction_sm() {
        const VLQ_AMOUNT: usize = 1_000_000;

        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);
        let input_vlqs: Box<[VlqData]> = (0..VLQ_AMOUNT).map(|_| random_vlq(&mut rng)).collect();
        let data: Box<[u8]> = vlq_multi_iter(&input_vlqs).collect();

        let mut feeder = ByteFeeder::new(&data);

        let mut sm = VlqReader::new();
        let mut output_vlqs = Vec::with_capacity(input_vlqs.len());

        while !feeder.is_empty() {
            sm.update_to_vec(feeder.bite(&mut rng), &mut output_vlqs)
                .expect("sm shouldn't error");
        }

        assert_eq!(*input_vlqs, *output_vlqs);
    }
}
