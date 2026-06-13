//! VLQ decoding and encoding structs.
//!
//! [`VlqData`]: used to represent a value as a VLQ byte string.\
//! [`VlqReader`]: used to convert a VLQ stream into a stream of [`VlqData`]s.

#![no_std]

extern crate alloc;

mod data;
mod reader;

pub use data::VlqData;
pub use reader::{VlqDecodeError, VlqReader};

#[cfg(test)]
mod test_utils {
    use core::ops::Deref;

    use crate::VlqData;
    use fastrand::Rng;

    pub const TEST_CHUNK_MAX_SIZE: usize = 48;

    /// A struct to split an input data into randomly-sized chunks.
    pub struct ByteFeeder<'a> {
        /// The slice representing the yet-to-be-output data.
        data: &'a [u8],
    }

    impl<'a> ByteFeeder<'a> {
        /// Creates a new byte feeder.
        #[must_use]
        pub const fn new(data: &'a [u8]) -> Self {
            Self { data }
        }

        /// Get a randomly-sized chunk of data.
        #[must_use]
        pub fn bite(&mut self, rng: &mut Rng) -> &'a [u8] {
            let chunk_size = rng.usize(1..=(TEST_CHUNK_MAX_SIZE.min(self.data.len())));
            let (chunk, rest) = self.data.split_at(chunk_size);
            self.data = rest;
            chunk
        }
    }

    impl<'a> Deref for ByteFeeder<'a> {
        type Target = &'a [u8];

        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }

    pub fn random_vlq(rng: &mut Rng) -> VlqData {
        let max = match rng.u8(..4) {
            0 => 2,
            1 => const { u8::MAX as u64 },
            2 => const { u16::MAX as u64 },
            _ => VlqData::MAX_REPRESENTABLE,
        };

        let num = rng.u64(..=max);

        // SAFETY: `num` is always at most the maximum representable by VlqData
        unsafe { VlqData::try_from(num).unwrap_unchecked() }
    }
}
