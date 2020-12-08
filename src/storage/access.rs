use super::layout::LayoutTrait;
use algebra::{CanonicalDeserialize, CanonicalSerialize};
use cfx_storage::{
    storage_db::{KeyValueDbTrait, KeyValueDbTraitRead},
    KvdbRocksdb,
};

use std::collections::HashMap;
use std::marker::PhantomData;

pub use error::Result;

pub struct TreeAccess<
    T: Default + Clone + CanonicalSerialize + CanonicalDeserialize,
    L: LayoutTrait,
> {
    // TODO: Maybe add a cache plan later.
    tree_name: String,
    db: KvdbRocksdb,
    cache: HashMap<L::Index, T>,
    _phantom: PhantomData<L>,
}

impl<T: Default + Clone + CanonicalSerialize + CanonicalDeserialize, L: LayoutTrait>
    TreeAccess<T, L>
{
    pub fn new(tree_name: String, db: KvdbRocksdb) -> Self {
        Self {
            tree_name,
            db,
            cache: HashMap::<L::Index, T>::new(),
            _phantom: PhantomData,
        }
    }

    fn compute_key(&self, node_index: &L::Index) -> Vec<u8> {
        let layout_index = <L as LayoutTrait>::position(node_index) as u32;

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

    pub fn entry(&mut self, node_index: &L::Index) -> &mut T {
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
    type NodeIndex = crate::amt::NodeIndex;
    type FlattenTree = super::FlattenTree;

    const TMP_RATIO: usize = 719323;

    let db = super::open_col("./__backend_tree", 0u32);
    let mut tree = TreeAccess::<u64, FlattenTree>::new("test".to_string(), db);

    for depth in 0..DEPTHS {
        for index in 0..(1 << depth) {
            let node_index = &NodeIndex::new(depth, index, DEPTHS);
            *tree.entry(node_index) = (TMP_RATIO * depth) as u64;
            *tree.entry(node_index) += index as u64;
        }
    }

    tree.flush();

    for depth in 0..DEPTHS {
        for index in 0..(1 << depth) {
            let node_index = &NodeIndex::new(depth, index, DEPTHS);
            assert_eq!((TMP_RATIO * depth + index) as u64, *tree.entry(node_index))
        }
    }

    drop(tree);

    ::std::fs::remove_dir_all("./__backend_tree").unwrap();
}

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
