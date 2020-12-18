use super::{key::Key, DEPTHS};
use crate::amt::tree::AMTName;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct TreeName(pub(super) usize, pub(super) u128);

impl TreeName {
    pub fn root() -> Self {
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

impl AMTName for TreeName {
    fn to_bytes(&self, depths: usize) -> Vec<u8> {
        let TreeName(level, index) = self.clone();
        if level == 0 {
            return vec![0u8];
        }

        let mut index_bytes = index.to_be_bytes();
        let chunks = (level * depths + 7) / 8;
        if chunks < 16 {
            index_bytes[15 - chunks] = level as u8;
            return index_bytes[(15 - chunks)..].to_vec();
        } else {
            let mut answer = vec![level as u8];
            answer.extend_from_slice(&index_bytes);
            return answer;
        }
    }
}

#[test]
fn test_tree_name_string() {
    const SMALL_DEPTHS: usize = 6;
    const MIDDLE_DEPTHS: usize = 16;
    const LARGE_DEPTHS: usize = 20;

    assert_eq!(TreeName(0, 0).to_bytes(SMALL_DEPTHS), [0u8]);

    assert_eq!(TreeName(1, 0).to_bytes(SMALL_DEPTHS), [1u8, 0u8]);
    assert_eq!(TreeName(1, 0).to_bytes(MIDDLE_DEPTHS), [1u8, 0u8, 0u8]);
    assert_eq!(TreeName(1, 0).to_bytes(LARGE_DEPTHS), [1u8, 0u8, 0u8, 0u8]);

    assert_eq!(TreeName(1, 1).to_bytes(SMALL_DEPTHS), [1u8, 1u8]);
    assert_eq!(TreeName(1, 1).to_bytes(MIDDLE_DEPTHS), [1u8, 0u8, 1u8]);
    assert_eq!(TreeName(1, 1).to_bytes(LARGE_DEPTHS), [1u8, 0u8, 0u8, 1u8]);

    assert_eq!(
        TreeName(2, 1024).to_bytes(LARGE_DEPTHS),
        [2u8, 0u8, 0u8, 0u8, 4u8, 0u8]
    );
}
