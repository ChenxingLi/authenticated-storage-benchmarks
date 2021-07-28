use crate::impl_storage_from_canonical;
use crate::{
    crypto::{
        export::{CanonicalDeserialize, CanonicalSerialize, ProjectiveCurve, SerializationError},
        TypeUInt,
    },
    storage::{StorageDecodable, StorageEncodable},
};
use std::io::{Read, Write};
use std::marker::PhantomData;

#[derive(Clone, Copy, Default, CanonicalDeserialize, CanonicalSerialize)]
pub struct AMTNode<G: ProjectiveCurve> {
    pub commitment: G,
    pub proof: G,
}

impl_storage_from_canonical!(AMTNode<T> where T: ProjectiveCurve);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct NodeIndex<N: TypeUInt> {
    depth: usize,
    index: usize,
    _phantom: PhantomData<N>,
}

impl<N: TypeUInt> NodeIndex<N> {
    #[inline]
    pub(crate) fn new(depth: usize, index: usize) -> Self {
        assert!(index < (1 << depth));
        assert!(depth <= N::USIZE);
        Self {
            depth,
            index,
            _phantom: PhantomData,
        }
    }

    pub fn root() -> Self {
        NodeIndex::new(0, 0)
    }

    #[inline]
    pub fn to_sibling(&self) -> Self {
        NodeIndex::new(self.depth, self.index ^ 1)
    }

    #[inline]
    pub fn to_ancestor(&self, height: usize) -> Self {
        assert!(height <= self.depth);
        NodeIndex::new(self.depth - height, self.index >> height)
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn total_depth(&self) -> usize {
        N::USIZE
    }
}
