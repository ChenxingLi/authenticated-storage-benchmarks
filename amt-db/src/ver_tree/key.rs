use super::DEPTHS;

use algebra::{CanonicalDeserialize, CanonicalSerialize, Read, SerializationError, Write};
use std::cmp::min;
use std::convert::TryFrom;

#[derive(
    Default, Hash, PartialEq, Eq, Clone, PartialOrd, Ord, CanonicalDeserialize, CanonicalSerialize,
)]
pub struct Key(Vec<u8>);

impl AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

const fn mask(length: usize) -> u128 {
    (1 << length) - 1
}

impl Key {
    #[inline]
    fn mid(&self, start: usize, length: usize) -> u128 {
        if length == 0 {
            return 0;
        }
        assert!(length <= 120);
        assert!(start < 8 * self.0.len());

        let start_byte = start / 8;
        let start_bit = start - start_byte * 8;

        let mut entry = self.0[start_byte..min(start_byte + 16, self.0.len())].to_vec();
        if entry.len() != 16 {
            entry.resize(16, 0);
        }
        let entry = u128::from_be_bytes(<[u8; 16]>::try_from(entry).unwrap());

        return entry >> start_bit & mask(length);
    }

    pub fn tree_at_level(&self, level: usize) -> u128 {
        let length = level * DEPTHS;
        self.mid(0, length)
    }

    pub fn index_at_level(&self, level: usize) -> u128 {
        let length = level * DEPTHS;
        self.mid(length, length + DEPTHS)
    }
}
