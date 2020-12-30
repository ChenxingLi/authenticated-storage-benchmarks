use crate::crypto::{
    paring_provider::{G1Aff, G1},
    serialize_length,
};
use crate::storage::{StorageDecodable, StorageEncodable, StoreByCanonicalSerialize};
use algebra::{
    BigInteger, CanonicalDeserialize, CanonicalSerialize, FromBytes, PairingEngine, PrimeField,
    SerializationError, ToBytes,
};
use std::io::{Read, Write};

#[derive(Clone, Copy)]
pub struct AMTNode<PE: PairingEngine> {
    pub commitment: G1<PE>,
    pub proof: G1<PE>,
}

impl<PE: PairingEngine> StorageEncodable for AMTNode<PE> {
    fn storage_encode(&self) -> Vec<u8> {
        let mut answer = Vec::with_capacity(2 * serialize_length::<G1<PE>>());
        self.commitment.write(&mut answer).unwrap();
        self.proof.write(&mut answer).unwrap();

        answer
    }
}

impl<PE: PairingEngine> StorageDecodable for AMTNode<PE> {
    fn storage_decode(mut data: &[u8]) -> Self {
        Self {
            commitment: FromBytes::read(&mut data).unwrap(),
            proof: FromBytes::read(&mut data).unwrap(),
        }
    }
}

impl<PE: PairingEngine> Default for AMTNode<PE> {
    fn default() -> Self {
        Self {
            commitment: G1::<PE>::default(),
            proof: G1::<PE>::default(),
        }
    }
}

impl<PE: PairingEngine> AMTNode<PE> {
    pub fn inc(&mut self, commitment: &G1<PE>, proof: &G1<PE>) {
        self.commitment += commitment;
        self.proof += proof;
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

    pub fn root(total_depth: usize) -> Self {
        NodeIndex::new(0, 0, total_depth)
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

    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn total_depth(&self) -> usize {
        self.total_depth
    }
}
