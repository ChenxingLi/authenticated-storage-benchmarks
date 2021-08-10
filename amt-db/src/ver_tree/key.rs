use super::DEPTHS;

use crate::crypto::export::{
    CanonicalDeserialize, CanonicalSerialize, Read, SerializationError, Write,
};
use std::cmp::min;
use std::convert::TryFrom;

#[derive(
    Default,
    Debug,
    Hash,
    PartialEq,
    Eq,
    Clone,
    PartialOrd,
    Ord,
    CanonicalDeserialize,
    CanonicalSerialize,
)]
pub struct Key(pub Vec<u8>);

impl AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Key {
    #[inline]
    fn mid(&self, start: usize, length: usize) -> u128 {
        if length == 0 {
            return 0;
        }

        let start_byte = start / 8;
        let start_bit = start - start_byte * 8;

        let mut entry = self.0[start_byte..min(start_byte + 16, self.0.len())].to_vec();

        if entry.len() != 16 {
            entry.resize(16, 0);
        }

        let entry = u128::from_be_bytes(<[u8; 16]>::try_from(entry).unwrap());

        return entry >> (start_bit + (128 - length));
    }

    pub fn tree_at_level(&self, level: u8) -> Vec<u32> {
        (0..level).map(|level| self.index_at_level(level)).collect()
    }

    pub fn index_at_level(&self, level: u8) -> u32 {
        let length = (level as usize) * DEPTHS;
        self.mid(length, DEPTHS) as u32
    }
}
