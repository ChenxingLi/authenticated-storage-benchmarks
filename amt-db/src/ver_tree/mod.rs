mod key;
mod name;
mod node;
mod tree;

use self::{
    key::Key,
    name::TreeName,
    node::{Node, MAX_VERSION_NUMBER},
};
use crate::{
    amt::{AMTConfigTrait, AMTree},
    crypto::paring_provider::{Pairing, G1},
    storage::{FlattenArray, FlattenTree},
};

#[derive(Copy, Clone)]
pub struct AMTConfig;

impl AMTConfigTrait for AMTConfig {
    type PE = Pairing;
    type Name = TreeName;
    type Data = Node;
    type DataLayout = FlattenArray;
    type TreeLayout = FlattenTree;

    const DEPTHS: usize = crate::crypto::DEPTHS;
}

const DEPTHS: usize = <AMTConfig as AMTConfigTrait>::DEPTHS;
const IDX_MASK: usize = <AMTConfig as AMTConfigTrait>::IDX_MASK;

type Tree = AMTree<AMTConfig>;
type Commitment = G1<<AMTConfig as AMTConfigTrait>::PE>;
