use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

pub(crate) const ROOT_INDEX: (usize, usize) = (0, 0);

pub type FlattenCompleteTree<T> = CompleteTree<T, FlattenLayout>;

pub trait LayoutIndex {
    fn layout_index(index: usize, depth: usize, total_depth: usize) -> usize;
}

pub struct FlattenLayout;

impl LayoutIndex for FlattenLayout {
    #[inline]
    fn layout_index(index: usize, depth: usize, total_depth: usize) -> usize {
        assert!(index < (1 << (total_depth + 1)));
        assert!(depth <= total_depth);

        (1 << depth) + index
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

impl<T: Default + Clone, L: LayoutIndex> IndexMut<(usize, usize)> for CompleteTree<T, L> {
    fn index_mut(&mut self, (depth, index): (usize, usize)) -> &mut Self::Output {
        let layout_index = <L as LayoutIndex>::layout_index(index, depth, self.total_depth);
        return &mut self.data[layout_index];
    }
}

impl<T: Default + Clone, L: LayoutIndex> Index<(usize, usize)> for CompleteTree<T, L> {
    type Output = T;

    fn index(&self, (depth, index): (usize, usize)) -> &Self::Output {
        let layout_index = <L as LayoutIndex>::layout_index(index, depth, self.total_depth);
        return &self.data[layout_index];
    }
}
