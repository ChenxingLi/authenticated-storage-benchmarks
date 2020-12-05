use algebra::{CanonicalDeserialize, CanonicalSerialize};
use cfx_storage::storage_db::{KeyValueDbTrait, KeyValueDbTraitRead};
use cfx_storage::KvdbRocksdb;

use std::collections::HashMap;
use std::marker::PhantomData;

use super::node::{LayoutIndex, NodeIndex};

mod error {
    use error_chain;
    error_chain! {
        links {
            RocksDB(cfx_storage::Error, cfx_storage::ErrorKind);
        }

        foreign_links {
            Serialize(algebra_core::serialize::SerializationError);
        }
    }
}

pub use error::Result;

pub struct TreeAccess<
    T: Default + Clone + CanonicalSerialize + CanonicalDeserialize,
    L: LayoutIndex,
> {
    tree_name: String,
    db: KvdbRocksdb,
    cache: HashMap<NodeIndex, T>,
    depth: usize,
    _phantom: PhantomData<L>,
}

impl<T: Default + Clone + CanonicalSerialize + CanonicalDeserialize, L: LayoutIndex>
    TreeAccess<T, L>
{
    pub fn new(tree_name: String, depth: usize, db: KvdbRocksdb) -> Self {
        Self {
            tree_name,
            db,
            cache: HashMap::<NodeIndex, T>::new(),
            depth,
            _phantom: PhantomData,
        }
    }

    fn compute_key(&self, node_index: &NodeIndex) -> Vec<u8> {
        assert!(node_index.depth() <= self.depth);
        let layout_index = <L as LayoutIndex>::layout_index(node_index, self.depth) as u32;

        let mut key = Vec::new();
        key.extend_from_slice(self.tree_name.as_bytes());
        key.extend_from_slice(&layout_index.to_be_bytes()); // We only use the last three bytes
        key
    }

    pub fn flush(&mut self) -> () {
        for (node, value) in self.cache.iter() {
            let mut serialized = vec![0; value.serialized_size()];
            value.serialize(&mut serialized[..]).unwrap();
            self.db.put(&self.compute_key(node), &serialized).unwrap();
        }
    }

    pub fn entry(&mut self, node_index: &NodeIndex) -> &mut T {
        if self.cache.contains_key(node_index) {
            self.cache.get_mut(node_index).unwrap()
        } else {
            let db_key = self.compute_key(node_index);
            let maybe_value = self.db.get(&db_key).unwrap();

            let value = match maybe_value {
                Some(x) => T::deserialize_unchecked(&*x).unwrap(),
                None => T::default(),
            };
            self.cache.entry(*node_index).or_insert(value)
        }
    }
}

#[test]
fn test_backend() {
    const LOCAL_DEPTH: usize = 3;
    const TMP_RATIO: usize = 719323;

    let db = crate::db::open_db("./__backend_tree", 0u32);
    let mut tree =
        TreeAccess::<u64, super::node::FlattenLayout>::new("test".to_string(), LOCAL_DEPTH, db);

    for depth in 0..LOCAL_DEPTH {
        for index in 0..(1 << depth) {
            let node_index = &NodeIndex::new(depth, index);
            *tree.entry(node_index) = (TMP_RATIO * depth) as u64;
            *tree.entry(node_index) += index as u64;
        }
    }

    tree.flush();

    for depth in 0..LOCAL_DEPTH {
        for index in 0..(1 << depth) {
            let node_index = &NodeIndex::new(depth, index);
            assert_eq!((TMP_RATIO * depth + index) as u64, *tree.entry(node_index))
        }
    }

    drop(tree);

    ::std::fs::remove_dir_all("./__backend_tree").unwrap();
}
