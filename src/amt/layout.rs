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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct NodeIndex {
    depth: usize,
    index: usize,
    total_depth: usize, // TODO: waiting for min-const-generic stabilized.
}

impl NodeIndex {
    #[inline]
    pub(crate) fn new(depth: usize, index: usize, total_depth: usize) -> Self {
        assert!(index < (1 << depth));
        assert!(depth <= total_depth);
        Self {
            depth,
            index,
            total_depth,
        }
    }

    #[inline]
    pub fn to_sibling(&self) -> Self {
        NodeIndex::new(self.depth, self.index ^ 1, self.total_depth)
    }

    #[inline]
    pub fn to_ancestor(&self, height: usize) -> Self {
        assert!(height <= self.depth);
        NodeIndex::new(self.depth - height, self.index >> height, self.total_depth)
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }
}

pub struct FlattenTree;

impl LayoutTrait for FlattenTree {
    type Index = NodeIndex;
    #[inline]
    fn position(tree_index: &NodeIndex) -> usize {
        (1 << tree_index.depth) + tree_index.index
    }
}
