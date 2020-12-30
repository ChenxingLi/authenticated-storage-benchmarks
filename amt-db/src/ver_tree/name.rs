use super::{key::Key, DEPTHS};
use crate::storage::{StorageDecodable, StorageEncodable};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct TreeName(pub(super) usize, pub(super) u128);

impl TreeName {
    pub const fn root() -> Self {
        TreeName(0, 0)
    }

    pub fn from_key_level(key: &Key, level: usize) -> Self {
        TreeName(level, key.tree_at_level(level))
    }

    pub fn parent(&self) -> Option<Self> {
        let TreeName(level, index) = self.clone();
        if level == 0 {
            None
        } else {
            Some(TreeName(level - 1, index >> DEPTHS))
        }
    }
}

impl StorageEncodable for TreeName {
    fn storage_encode(&self) -> Vec<u8> {
        let TreeName(level, index) = self.clone();
        if level == 0 {
            return vec![0u8];
        }

        let truncates = (index.leading_zeros() / 8) as usize;
        let mut index_bytes: [u8; 16] = index.to_be_bytes();
        return if truncates > 0 {
            index_bytes[truncates - 1] = level as u8;
            index_bytes[(truncates - 1)..].to_vec()
        } else {
            let mut answer = vec![level as u8];
            answer.extend_from_slice(&index_bytes);
            answer
        };
    }
}

impl StorageDecodable for TreeName {
    fn storage_decode(data: &[u8]) -> Self {
        let level = data[0] as usize;

        let index_length = data[1..].len();
        let mut index_bytes = [0u8; 16];
        index_bytes[(16 - index_length)..].copy_from_slice(&data[1..]);
        let index = u128::from_be_bytes(index_bytes);

        TreeName(level, index)
    }
}

#[test]
fn test_tree_name_string() {
    assert_eq!(TreeName(0, 0).storage_encode(), [0u8]);
    assert_eq!(TreeName(0, 0), TreeName::storage_decode(&[0u8]));

    assert_eq!(TreeName(1, 0).storage_encode(), [1u8]);
    assert_eq!(TreeName(1, 0), TreeName::storage_decode(&[1u8]));

    // assert_eq!(TreeName(0, 0).to_bytes(SMALL_DEPTHS), [0u8]);
    //
    // assert_eq!(TreeName(1, 0).to_bytes(SMALL_DEPTHS), [1u8, 0u8]);
    // assert_eq!(TreeName(1, 0).to_bytes(MIDDLE_DEPTHS), [1u8, 0u8, 0u8]);
    // assert_eq!(TreeName(1, 0).to_bytes(LARGE_DEPTHS), [1u8, 0u8, 0u8, 0u8]);
    //
    // assert_eq!(TreeName(1, 1).to_bytes(SMALL_DEPTHS), [1u8, 1u8]);
    // assert_eq!(TreeName(1, 1).to_bytes(MIDDLE_DEPTHS), [1u8, 0u8, 1u8]);
    // assert_eq!(TreeName(1, 1).to_bytes(LARGE_DEPTHS), [1u8, 0u8, 0u8, 1u8]);
    //
    // assert_eq!(
    //     TreeName(2, 1024).to_bytes(LARGE_DEPTHS),
    //     [2u8, 0u8, 0u8, 0u8, 4u8, 0u8]
    // );
}
