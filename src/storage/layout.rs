use crate::amt::NodeIndex;
use std::fmt::Debug;
use std::hash::Hash;

pub trait LayoutTrait {
    type Index: Copy + Clone + Debug + Eq + Hash;
    fn position(index: &Self::Index) -> usize;
}

pub struct FlattenArray;

impl LayoutTrait for FlattenArray {
    type Index = usize;
    #[inline]
    fn position(index: &usize) -> usize {
        *index
    }
}

pub struct FlattenTree;

impl LayoutTrait for FlattenTree {
    type Index = NodeIndex;
    #[inline]
    fn position(tree_index: &NodeIndex) -> usize {
        (1 << tree_index.depth()) + tree_index.index()
    }
}
