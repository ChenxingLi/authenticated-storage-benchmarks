use crate::crypto::{serialize_length, TypeUInt};
use crate::storage::{StorageDecodable, StorageEncodable};
use algebra::{FromBytes, ProjectiveCurve};
use std::marker::PhantomData;

#[derive(Clone, Copy, Default)]
pub struct AMTNode<G: ProjectiveCurve> {
    pub commitment: G,
    pub proof: G,
}

impl<G: ProjectiveCurve> StorageEncodable for AMTNode<G> {
    fn storage_encode(&self) -> Vec<u8> {
        let mut answer = Vec::with_capacity(2 * serialize_length::<G>());
        self.commitment.write(&mut answer).unwrap();
        self.proof.write(&mut answer).unwrap();

        answer
    }
}

impl<G: ProjectiveCurve> StorageDecodable for AMTNode<G> {
    fn storage_decode(mut data: &[u8]) -> Self {
        Self {
            commitment: FromBytes::read(&mut data).unwrap(),
            proof: FromBytes::read(&mut data).unwrap(),
        }
    }
}

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
