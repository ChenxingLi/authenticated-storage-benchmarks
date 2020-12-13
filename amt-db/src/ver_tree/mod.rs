mod key;
mod name;
mod node;
mod tree;

use self::{key::Key, name::TreeName, node::Node};
use crate::{
    amt::{paring_provider::Pairing, AMTConfigTrait, AMTree, DEPTHS},
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
}

type Tree = AMTree<AMTConfig>;
