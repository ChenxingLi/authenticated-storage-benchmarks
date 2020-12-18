pub mod config;
pub mod node;
pub mod tree;

#[cfg(test)]
mod test;

pub use self::{
    node::NodeIndex,
    tree::{AMTConfigTrait, AMTData, AMTree},
};
