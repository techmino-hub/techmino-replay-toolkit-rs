use crate::{vlq::VlqData, ReplayParseError};

// TODO: Rename this to just VlqReader
/// A VLQ reader state machine that can take in arbitrary chunks
/// of VLQ-encoded bytes.
#[derive(Clone, Debug)]
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

        self.buf = buf[..(VlqData::MAX_BYTES - 1) as usize].try_into().unwrap();

        Ok(())
    }

    /// Returns false if this struct has any leftover partial data.
    #[must_use]
    pub(crate) const fn is_finished(&self) -> bool {
        self.buf_len == 0
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
}

#[cfg(test)]
mod tests {
    use fastrand::Rng;

    use crate::tests::{random_vlq, ByteFeeder};

    use super::*;

    #[test]
    fn test_vlq_extraction_sm() {
        const VLQ_AMOUNT: usize = 1_000_000;

        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);
        let input_vlqs: Box<[VlqData]> = (0..VLQ_AMOUNT).map(|_| random_vlq(&mut rng)).collect();
        let data: Box<[u8]> = VlqData::multi_iter(&input_vlqs).collect();

        let mut feeder = ByteFeeder::new(&data);

        let mut sm = VlqReaderSM::new();
        let mut output_vlqs = Vec::with_capacity(input_vlqs.len());

        while !feeder.is_empty() {
            sm.update_to_vec(feeder.bite(&mut rng), &mut output_vlqs)
                .expect("sm shouldn't error");
        }

        assert_eq!(*input_vlqs, *output_vlqs);
    }
}
