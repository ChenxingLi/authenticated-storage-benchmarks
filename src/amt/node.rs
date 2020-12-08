use super::paring_provider::{G1Aff, G1};
use algebra::{CanonicalDeserialize, CanonicalSerialize, PairingEngine, SerializationError};
use std::io::{Read, Write};

#[derive(Clone, Copy)]
pub struct AMTNode<PE: PairingEngine> {
    pub commitment: G1<PE>,
    pub proof: G1<PE>,
}
type CompressedAMTNode<PE> = (G1Aff<PE>, G1Aff<PE>);

impl<PE: PairingEngine> Default for AMTNode<PE> {
    fn default() -> Self {
        Self {
            commitment: G1::<PE>::default(),
            proof: G1::<PE>::default(),
        }
    }
}

impl<PE: PairingEngine> From<CompressedAMTNode<PE>> for AMTNode<PE> {
    fn from((commitment, proof): CompressedAMTNode<PE>) -> Self {
        Self {
            commitment: G1::<PE>::from(commitment),
            proof: G1::<PE>::from(proof),
        }
    }
}

impl<PE: PairingEngine> Into<CompressedAMTNode<PE>> for AMTNode<PE> {
    fn into(self) -> CompressedAMTNode<PE> {
        (self.commitment.into(), self.proof.into())
    }
}

impl<PE: PairingEngine> AMTNode<PE> {
    pub fn inc(&mut self, commitment: &G1<PE>, proof: &G1<PE>) {
        self.commitment += commitment;
        self.proof += proof;
    }
}

// TODO: this is only an ad-hoc fix to make AMTNode Serializable.

impl<PE: PairingEngine> CanonicalDeserialize for AMTNode<PE> {
    fn deserialize<R: Read>(mut reader: R) -> Result<Self, SerializationError> {
        let compressed_node: CompressedAMTNode<PE> =
            CanonicalDeserialize::deserialize_unchecked(&mut reader)?;
        Ok(compressed_node.into())
    }
}

impl<PE: PairingEngine> CanonicalSerialize for AMTNode<PE> {
    fn serialize<W: Write>(&self, mut writer: W) -> Result<(), SerializationError> {
        let compressed_node: CompressedAMTNode<PE> = self.clone().into();
        compressed_node.serialize_unchecked(&mut writer)
    }

    fn serialized_size(&self) -> usize {
        let compressed_node: CompressedAMTNode<PE> = self.clone().into();
        compressed_node.uncompressed_size()
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

    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn total_depth(&self) -> usize {
        self.total_depth
    }
}
