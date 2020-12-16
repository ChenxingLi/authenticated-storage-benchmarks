use super::key::Key;
use crate::amt::DEPTHS;

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

impl From<TreeName> for Vec<u8> {
    fn from(_: TreeName) -> Self {
        "12".into()
    }
}
