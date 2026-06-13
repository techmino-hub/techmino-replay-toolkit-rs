use core::num::NonZeroU8;

/// A struct representing VLQ-encoded data.
///
/// Provides the bridge between raw `u64` values and
/// VLQ-encoded `[u8]`s.
///
/// The maximum size of the `[u8]` array is 8 bytes long,
/// which means it has a limit of representing up to
/// 56 bits (since one bit of each byte is used by the format).
#[derive(Clone, Debug)]
pub struct VlqData {
    /// The byte array representing the VLQ-encoded data.
    ///
    /// The first byte with the most significant bit ON
    /// (i.e., greater than 0x80) indicates the last byte of
    /// the VLQ. Anything past that shall be disregarded.
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
    pub const fn from_value(mut value: u64) -> Result<Self, VlqEncodeError> {
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

        Ok(Self { bytes })
    }

    /// Converts a bytes into a `VlqData`.
    ///
    /// # Unchecked Operation
    /// For this operation to be valid, there must be at least one byte in the input
    /// less than 0x80, and every byte before that (if any) must be greater than 0x80.
    ///
    /// Failing this constraint is a logic error.
    #[must_use]
    pub const fn from_raw(bytes: [u8; Self::MAX_BYTES as usize]) -> Self {
        debug_assert!(
            u64::from_ne_bytes(bytes) & 0x8080_8080_8080_8080 != 0x8080_8080_8080_8080,
            "invalid vlq: no ending byte found"
        );

        Self { bytes }
    }

    /// Gets the length of the underlying VLQ, where `data[..len]` is
    /// a valid VLQ slice.
    #[must_use]
    pub const fn len(&self) -> NonZeroU8 {
        // Bit twiddling!
        // 1. Filter out the mask
        // 2. Invert the mask (0x80 becomes 0x00, vice versa)
        // 3. Get the amt of bytes to the first 0x80
        // 4. Divide by 8 then add one to get answer

        let num = u64::from_be_bytes(self.bytes);

        let masked = num & 0x8080_8080_8080_8080;
        let inverted = masked ^ 0x8080_8080_8080_8080;

        #[expect(clippy::cast_possible_truncation, reason = "len can only be up to 8")]
        let len = (inverted.leading_zeros() / 8 + 1) as u8;

        unsafe { NonZeroU8::new_unchecked(len) }
    }

    #[must_use]
    pub const fn value(&self) -> u64 {
        let mut value = 0u64;

        let mut idx = 0;
        let len = self.len().get();

        while idx < len {
            let byte = self.bytes[idx as usize];

            value <<= 7;
            value |= (byte & 0x7F) as u64;

            idx += 1;
        }

        value
    }

    /// Get the VLQ-encoded bytes as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.bytes.get(..self.len().get() as usize).unwrap()
    }
}

impl TryFrom<u64> for VlqData {
    type Error = VlqEncodeError;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        VlqData::from_value(value)
    }
}

impl<'v> From<&'v VlqData> for &'v [u8] {
    fn from(value: &'v VlqData) -> Self {
        value.as_slice()
    }
}

impl PartialEq for VlqData {
    fn eq(&self, other: &Self) -> bool {
        let len = self.len();

        len == other.len() && self.bytes[..len.get() as usize] == other.bytes[..len.get() as usize]
    }
}

/// There was an attempt to encode an oversized `u64` into the VLQ format.
#[derive(Debug)]
pub struct VlqEncodeError {
    /// The `u64` value that couldn't be encoded into the VLQ format.
    #[expect(dead_code, reason = "this is purely for diagnostic purposes")]
    number: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastrand::Rng;

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
            let vlq = VlqData::from_value(value).unwrap();
            assert_eq!(vlq.as_slice(), expected_vlqs.as_slice());
        }
    }

    #[test]
    fn test_vlq_roundtrip() {
        for i in 0..u64::from(u16::MAX) {
            let vlq = VlqData::from_value(i).unwrap();

            assert_eq!(vlq.value(), i);
        }

        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..1_000_000 {
            let val = rng.u64(0..=VlqData::MAX_REPRESENTABLE);
            let vlq = VlqData::from_value(val).unwrap();
            assert_eq!(vlq.value(), val);
        }

        assert_eq!(
            VlqData::from_value(VlqData::MAX_REPRESENTABLE)
                .unwrap()
                .value(),
            VlqData::MAX_REPRESENTABLE
        );
    }

    #[test]
    fn test_vlq_len() {
        let mut rng = Rng::with_seed(0x4d59_5df4_d0f3_3173);

        for _ in 0..1_000_000 {
            let len = rng.u8(1..=8);
            let max = (1u64 << (7 * len)) - 1;

            let min = match len {
                1 => 0,
                _ => 1 << (7 * len - 1),
            };

            let value = rng.u64(min..=max);

            let vlq = match VlqData::from_value(value) {
                Ok(v) => v,
                Err(e) => {
                    panic!("Error encoding VLQ for value `{value:x}`: {e:?}")
                }
            };

            assert_eq!(vlq.len().get(), len);
        }
    }
}
