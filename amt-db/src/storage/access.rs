use super::{layout::LayoutTrait, StorageDecodable, StorageEncodable};
use cfx_storage::{
    storage_db::{KeyValueDbTrait, KeyValueDbTraitRead},
    KvdbRocksdb,
};

use hashbrown::HashMap;
use std::marker::PhantomData;

use global::Global;
use std::fmt::Debug;
use std::hash::Hash;

pub static PUT_COUNT: Global<[u64; 4]> = Global::INIT;
pub static PUT_MODE: Global<usize> = Global::INIT;

#[derive(Clone)]
pub struct DBAccess<
    K: Copy + Clone + Debug + Eq + Hash,
    V: Default + Clone + StorageEncodable + StorageDecodable,
    L: LayoutTrait<K>,
> {
    // TODO: Maybe add a cache plan later, and use a generic type for db access.
    name: Vec<u8>,
    db: KvdbRocksdb,
    cache: HashMap<K, (V, bool)>,
    dump_mode: bool,
    _phantom: PhantomData<L>,
}

impl<
        K: Copy + Clone + Debug + Eq + Hash,
        V: Default + Clone + StorageEncodable + StorageDecodable,
        L: LayoutTrait<K>,
    > DBAccess<K, V, L>
{
    pub fn new(name: Vec<u8>, db: KvdbRocksdb) -> Self {
        Self {
            name,
            db,
            cache: HashMap::new(),
            dump_mode: false,
            _phantom: PhantomData,
        }
    }

    pub fn get(&mut self, node_index: &K) -> &V {
        let (value, _dirty) = self.get_cached(node_index);
        value
    }

    pub fn get_mut(&mut self, node_index: &K) -> &mut V {
        let (value, dirty) = self.get_cached(node_index);
        *dirty = true;
        value
    }

    pub fn set(&mut self, node_index: &K, value: V) {
        self.cache.insert(*node_index, (value, true));
    }

    pub fn flush(&mut self) {
        for (key, (value, dirty)) in self.cache.iter_mut().filter(|(_k, (_v, dirty))| *dirty) {
            (*PUT_COUNT.lock_mut().unwrap())[*PUT_MODE.lock().unwrap()] += 1;
            let db_key = Self::compute_key(&self.name, key);
            self.db.put(&db_key, &value.storage_encode()).unwrap();
            *dirty = false;
        }
        self.cache.clear();
    }

    fn get_cached(&mut self, node_index: &K) -> &mut (V, bool) {
        let (name, db) = (&self.name, &self.db);
        self.cache.entry(*node_index).or_insert_with(|| {
            let db_key = Self::compute_key(&name, node_index);

            let value = match db.get(&db_key).unwrap() {
                Some(x) => V::storage_decode(&*x).unwrap(),
                None => V::default(),
            };
            (value, false)
        })
    }

    fn compute_key(name: &[u8], node_index: &K) -> Vec<u8> {
        let layout_index = <L as LayoutTrait<K>>::position(node_index) as u32;

        let mut key = name.to_vec();
        key.extend_from_slice(&layout_index.to_be_bytes());
        key
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::crypto::TypeUInt;
    use crate::type_uint;

    type_uint! {
        struct TestDepths(6);
    }

    #[test]
    fn test_backend() {
        type NodeIndex = crate::amt::NodeIndex<TestDepths>;
        type FlattenTree = super::super::FlattenTree;

        const DEPTHS: usize = TestDepths::USIZE;
        const TMP_RATIO: usize = 719323;

        let db = super::super::open_col("./__backend_tree", 0u32);
        let mut tree =
            DBAccess::<NodeIndex, u64, FlattenTree>::new("test".to_string().into_bytes(), db);

        for depth in 0..DEPTHS {
            for index in 0..(1 << depth) {
                let node_index = &NodeIndex::new(depth, index);
                *tree.get_mut(node_index) = (TMP_RATIO * depth) as u64;
                *tree.get_mut(node_index) += index as u64;
            }
        }

        tree.flush();

        for depth in 0..DEPTHS {
            for index in 0..(1 << depth) {
                let node_index = &NodeIndex::new(depth, index);
                assert_eq!(
                    (TMP_RATIO * depth + index) as u64,
                    *tree.get_mut(node_index)
                )
            }
        }

        drop(tree);

        std::fs::remove_dir_all("./__backend_tree").unwrap();
    }
}
