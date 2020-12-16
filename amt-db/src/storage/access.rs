use super::layout::LayoutTrait;
use algebra::{CanonicalDeserialize, CanonicalSerialize};
use cfx_storage::{
    storage_db::{KeyValueDbTrait, KeyValueDbTraitRead},
    KvdbRocksdb,
};

use std::collections::HashMap;
use std::marker::PhantomData;

pub use error::Result;
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Clone)]
pub struct TreeAccess<
    K: Copy + Clone + Debug + Eq + Hash,
    V: Default + Clone + CanonicalSerialize + CanonicalDeserialize,
    L: LayoutTrait<K>,
> {
    // TODO: Maybe add a cache plan later, and use a generic type for db access.
    tree_name: Vec<u8>,
    db: KvdbRocksdb,
    cache: HashMap<K, (V, bool)>,
    _phantom: PhantomData<L>,
}

impl<
        K: Copy + Clone + Debug + Eq + Hash,
        V: Default + Clone + CanonicalSerialize + CanonicalDeserialize,
        L: LayoutTrait<K>,
    > TreeAccess<K, V, L>
{
    pub fn new(tree_name: Vec<u8>, db: KvdbRocksdb) -> Self {
        Self {
            tree_name,
            db,
            cache: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    pub fn get(&mut self, node_index: &K) -> &V {
        let (value, _dirty) = self.cache_and_get(node_index);
        value
    }

    pub fn get_mut(&mut self, node_index: &K) -> &mut V {
        let (value, dirty) = self.cache_and_get(node_index);
        *dirty = true;
        value
    }

    pub fn flush(&mut self) {
        for (key, (value, dirty)) in self.cache.iter_mut().filter(|(_k, (_v, dirty))| *dirty) {
            let mut serialized = vec![0; value.serialized_size()];
            value.serialize(&mut serialized[..]).unwrap();

            let db_key = Self::compute_key(&self.tree_name, key);
            self.db.put(&db_key, &serialized).unwrap();
            *dirty = false;
        }
    }

    fn cache_and_get(&mut self, node_index: &K) -> &mut (V, bool) {
        if !self.cache.contains_key(node_index) {
            let db_key = Self::compute_key(&self.tree_name, node_index);
            let maybe_value = self.db.get(&db_key).unwrap();

            let value = match maybe_value {
                Some(x) => V::deserialize_unchecked(&*x).unwrap(),
                None => V::default(),
            };
            self.cache.entry(*node_index).or_insert((value, false))
        } else {
            self.cache.get_mut(node_index).unwrap()
        }
    }

    fn compute_key(name: &[u8], node_index: &K) -> Vec<u8> {
        let layout_index = <L as LayoutTrait<K>>::position(node_index) as u32;

        let mut key = Vec::new();
        key.extend_from_slice(name);
        key.extend_from_slice(&layout_index.to_be_bytes());
        key
    }
}

#[test]
fn test_backend() {
    type NodeIndex = crate::amt::NodeIndex;
    type FlattenTree = super::FlattenTree;

    const DEPTHS: usize = crate::amt::DEPTHS;
    const TMP_RATIO: usize = 719323;

    let db = super::open_col("./__backend_tree", 0u32);
    let mut tree =
        TreeAccess::<NodeIndex, u64, FlattenTree>::new("test".to_string().into_bytes(), db);

    for depth in 0..DEPTHS {
        for index in 0..(1 << depth) {
            let node_index = &NodeIndex::new(depth, index, DEPTHS);
            *tree.get_mut(node_index) = (TMP_RATIO * depth) as u64;
            *tree.get_mut(node_index) += index as u64;
        }
    }

    tree.flush();

    for depth in 0..DEPTHS {
        for index in 0..(1 << depth) {
            let node_index = &NodeIndex::new(depth, index, DEPTHS);
            assert_eq!(
                (TMP_RATIO * depth + index) as u64,
                *tree.get_mut(node_index)
            )
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
