#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum TreeName {
    Root,
    Subtree(usize, u64),
}

impl From<TreeName> for String {
    fn from(_: TreeName) -> Self {
        "12".into()
    }
}
