use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct NodeIndex {
    depth: usize,
    index: usize,
}

impl NodeIndex {
    pub(crate) fn new(depth: usize, index: usize) -> Self {
        assert!(index < (1 << depth));
        Self { depth, index }
    }

    pub fn to_sibling(&self) -> Self {
        NodeIndex::new(self.depth, self.index ^ 1)
    }

    pub fn to_ancestor(&self, height: usize) -> Self {
        assert!(height <= self.depth);
        NodeIndex::new(self.depth - height, self.index >> height)
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

pub type FlattenCompleteTree<T> = CompleteTree<T, FlattenLayout>;

pub trait LayoutIndex {
    fn layout_index(index: NodeIndex, total_depth: usize) -> usize;
}

pub struct FlattenLayout;

impl LayoutIndex for FlattenLayout {
    #[inline]
    fn layout_index(tree_index: NodeIndex, total_depth: usize) -> usize {
        assert!(tree_index.depth <= total_depth);
        (1 << tree_index.depth) + tree_index.index
    }
}

pub struct CompleteTree<T: Default + Clone, L: LayoutIndex> {
    total_depth: usize,
    data: Vec<T>,
    _phantom: PhantomData<L>,
}

impl<T: Default + Clone, L: LayoutIndex> CompleteTree<T, L> {
    pub fn new(total_depth: usize) -> Self {
        Self {
            total_depth,
            data: vec![T::default(); 2 * (1 << total_depth)],
            _phantom: PhantomData::default(),
        }
    }
}

impl<T: Default + Clone, L: LayoutIndex> IndexMut<NodeIndex> for CompleteTree<T, L> {
    fn index_mut(&mut self, tree_index: NodeIndex) -> &mut Self::Output {
        let layout_index = <L as LayoutIndex>::layout_index(tree_index, self.total_depth);
        return &mut self.data[layout_index];
    }
}

impl<T: Default + Clone, L: LayoutIndex> Index<NodeIndex> for CompleteTree<T, L> {
    type Output = T;

    fn index(&self, tree_index: NodeIndex) -> &Self::Output {
        let layout_index = <L as LayoutIndex>::layout_index(tree_index, self.total_depth);
        return &self.data[layout_index];
    }
}
