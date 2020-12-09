use super::node::VerNode;
use super::Key;
use crate::{
    amt::{paring_provider::Pairing, AMTree},
    storage::{FlattenArray, KvdbRocksdb, SystemDB, TreeAccess},
};
use std::collections::HashMap;
use std::sync::Arc;
//
// fn prefix(data: &[u8], length: usize) -> Vec<u8> {
//     let bytes = (length + 7) / 8;
//     let rest = bytes * 8 - length;
//     let mut answer = data[..bytes].to_vec();
//     answer[bytes - 1] &= (1 << 8 - rest) - 1;
//     answer
// }
//
pub struct VerForest {
    db: Arc<SystemDB>,
    forest: HashMap<Key, VerTree>,
}
