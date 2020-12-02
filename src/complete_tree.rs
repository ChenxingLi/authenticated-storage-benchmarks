use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

pub(crate) const ROOT_INDEX: (usize, usize) = (0, 0);

pub type FlattenCompleteTree<T> = CompleteTree<T, FlattenLayout>;

pub trait LayoutIndex {
    fn layout_index(index: usize, level: usize, total_level: usize) -> usize;
}

pub struct FlattenLayout;

impl LayoutIndex for FlattenLayout {
    #[inline]
    fn layout_index(index: usize, level: usize, total_level: usize) -> usize {
        assert!(index < (1 << (total_level + 1)));
        assert!(level <= total_level);

        (1 << level) + index
    }
}

pub struct CompleteTree<T: Default + Clone, L: LayoutIndex> {
    total_level: usize,
    data: Vec<T>,
    _phantom: PhantomData<L>,
}

impl<T: Default + Clone, L: LayoutIndex> CompleteTree<T, L> {
    pub fn new(total_level: usize) -> Self {
        Self {
            total_level,
            data: vec![T::default(); 2 * (1 << total_level)],
            _phantom: PhantomData::default(),
        }
    }
}

impl<T: Default + Clone, L: LayoutIndex> IndexMut<(usize, usize)> for CompleteTree<T, L> {
    fn index_mut(&mut self, (level, index): (usize, usize)) -> &mut Self::Output {
        let layout_index = <L as LayoutIndex>::layout_index(index, level, self.total_level);
        return &mut self.data[layout_index];
    }
}

impl<T: Default + Clone, L: LayoutIndex> Index<(usize, usize)> for CompleteTree<T, L> {
    type Output = T;

    fn index(&self, (level, index): (usize, usize)) -> &Self::Output {
        let layout_index = <L as LayoutIndex>::layout_index(index, level, self.total_level);
        return &self.data[layout_index];
    }
}
