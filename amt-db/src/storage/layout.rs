use crate::amt::NodeIndex;
use std::fmt::Debug;
use std::hash::Hash;

pub trait LayoutTrait<I: Copy + Clone + Debug + Eq + Hash> {
    fn position(index: &I) -> usize;
}

#[derive(Clone)]
pub struct FlattenArray;

impl LayoutTrait<usize> for FlattenArray {
    #[inline]
    fn position(index: &usize) -> usize {
        *index
    }
}

#[derive(Clone)]
pub struct FlattenTree;

impl LayoutTrait<NodeIndex> for FlattenTree {
    #[inline]
    fn position(tree_index: &NodeIndex) -> usize {
        (1 << tree_index.depth()) + tree_index.index()
    }
}
