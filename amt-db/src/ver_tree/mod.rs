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
    amt::{
        paring_provider::{Pairing, G1},
        AMTConfigTrait, AMTree, DEPTHS,
    },
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

    const DEPTHS: usize = DEPTHS;
}

type Tree = AMTree<AMTConfig>;
type Commitment = G1<<AMTConfig as AMTConfigTrait>::PE>;
