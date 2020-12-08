use super::node::VerNode;
use crate::amt::paring_provider::Pairing;
use crate::amt::AMTree;
use crate::storage::{FlattenArray, KvdbRocksdb, SystemDB, TreeAccess};
use std::sync::Arc;

fn prefix(data: &[u8], length: usize) -> Vec<u8> {
    let bytes = (length + 7) / 8;
    let rest = bytes * 8 - length;
    let mut answer = data[..bytes].to_vec();
    answer[bytes - 1] &= (1 << 8 - rest) - 1;
    answer
}

pub struct VerTree {
    inner: AMTree<Pairing>,
    nodes: TreeAccess<VerNode, FlattenArray>,
}

impl VerTree {
    pub fn new(name: String, db: Arc<SystemDB>) -> Self {
        Self {
            inner: AMTree::<Pairing>::new(name, db),
            nodes: TreeAccess::new(
                format!("data:{}", name),
                KvdbRocksdb {
                    kvdb: db.key_value().clone(),
                    col: 0,
                },
            ),
        }
    }
}
