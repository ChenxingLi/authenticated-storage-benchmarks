use algebra::{CanonicalDeserialize, CanonicalSerialize, PairingEngine, SerializationError};
use std::io::{Read, Write};

type G1<PE> = <PE as PairingEngine>::G1Projective;
type G1Aff<PE> = <PE as PairingEngine>::G1Affine;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
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
}

#[derive(Clone, Copy)]
pub(crate) struct AMTNode<PE: PairingEngine> {
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

pub struct FlattenLayout;

impl LayoutIndex for FlattenLayout {
    #[inline]
    fn layout_index(tree_index: &NodeIndex, total_depth: usize) -> usize {
        assert!(tree_index.depth <= total_depth);
        (1 << tree_index.depth) + tree_index.index
    }
}

pub trait LayoutIndex {
    fn layout_index(index: &NodeIndex, total_depth: usize) -> usize;
}
