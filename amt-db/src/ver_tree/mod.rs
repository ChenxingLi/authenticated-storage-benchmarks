mod key;
mod name;
mod node;
mod tree;

#[cfg(test)]
mod test;

pub use self::{
    key::Key,
    name::TreeName,
    node::{Node, MAX_VERSION_NUMBER},
    tree::VerForest,
};
use crate::{
    amt::{AMTConfigTrait, AMTree},
    crypto::paring_provider::{Pairing, G1},
    storage::{FlattenArray, FlattenTree, StoreTupleByBytes},
};

#[derive(Copy, Clone)]
pub struct AMTConfig;

impl AMTConfigTrait for AMTConfig {
    type PE = Pairing;
    type Name = TreeName;
    type Data = Node;
    type DataLayout = FlattenArray;
    type TreeLayout = FlattenTree;
    type Height = crate::crypto::TypeDepths;
}

type Tree = AMTree<AMTConfig>;
pub type Commitment = G1<<AMTConfig as AMTConfigTrait>::PE>;

const DEPTHS: usize = <AMTConfig as AMTConfigTrait>::DEPTHS;
const IDX_MASK: usize = <AMTConfig as AMTConfigTrait>::IDX_MASK;

impl StoreTupleByBytes for (TreeName, Commitment) {}

// impl StorageEncodable for (TreeName, Commitment) {
//     fn storage_encode(&self) -> Vec<u8> {
//         let (TreeName(level, position), commitment) = self;
//         let commitment_affine = commitment.into_affine();
//         let mut serialized = vec![0; 17 + commitment_affine.serialized_size()];
//         serialized[0] = *level as u8;
//         serialized[1..17].copy_from_slice(&position.to_be_bytes());
//         commitment_affine
//             .serialize_unchecked(&mut serialized[17..])
//             .unwrap();
//         serialized
//     }
// }

// impl<T: CanonicalDeserialize> StorageDecodable for T {
//     fn storage_decode(data: Box<[u8]>) -> Self {
//         Self::deserialize_unchecked(&*data).unwrap()
//     }
// }
