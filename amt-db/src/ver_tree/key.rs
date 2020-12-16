use super::DEPTHS;

use algebra::{CanonicalDeserialize, CanonicalSerialize, Read, SerializationError, Write};

#[derive(Default, Hash, PartialEq, Eq, Clone, CanonicalDeserialize, CanonicalSerialize)]
pub struct Key(Vec<u64>);

const fn mask(length: usize) -> u128 {
    (1 << length) - 1
}

impl Key {
    fn mid(&self, start: usize, length: usize) -> u128 {
        if length == 0 {
            return 0;
        }
        assert!(length <= 128);

        let start_chunk = start / 64;
        let start_bit = start - start_chunk * 64;

        let entry_u128 = |index: usize| self.0.get(index).copied().unwrap_or(0) as u128;

        let part1 = entry_u128(start_chunk) << (start_bit + 64);
        let part2 = entry_u128(start_chunk + 1) << start_bit;
        let part3 = entry_u128(start_chunk + 1) >> (64 - start_bit);

        return (part1 | part2 | part3) >> (192 - length - start_bit) & mask(length);
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
