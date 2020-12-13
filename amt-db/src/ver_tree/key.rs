use super::DEPTHS;

use algebra::{CanonicalDeserialize, CanonicalSerialize, Read, SerializationError, Write};

#[derive(Default, Hash, PartialEq, Eq, Clone, CanonicalDeserialize, CanonicalSerialize)]
pub struct Key(Vec<u64>);

const fn mask(len: usize) -> u64 {
    (1 << len) - 1
}

impl Key {
    fn mid(&self, start: usize, length: usize) -> u64 {
        if length == 0 {
            return 0;
        }
        assert!(length <= 64);
        let start_chunk = start / 64;
        let end_chunk = (start + length) / 64;
        let start_bit = start - start_chunk * 64;
        return if start_chunk == end_chunk {
            self.0[start_chunk] >> start_bit & mask(length)
        } else {
            let answer = self.0[start_chunk] >> start_bit & mask(length);
            let rest = self.0[start_chunk] & mask(start_bit + length - 64);
            answer | (rest << (64 - start_bit))
        };
    }

    pub fn tree_at_level(&self, level: usize) -> u64 {
        let length = level * DEPTHS;
        self.mid(256 - length, length)
    }

    pub fn index_at_level(&self, level: usize) -> u64 {
        let length = level * DEPTHS;
        self.mid(256 - length - DEPTHS, DEPTHS)
    }
}
