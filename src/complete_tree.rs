use algebra::{CanonicalDeserialize, CanonicalSerialize};
use algebra_core::{FromBytes, ToBytes};
use cfx_storage::storage_db::{KeyValueDbTrait, KeyValueDbTraitRead};
use cfx_storage::KvdbRocksdb;

use super::db::open_db;
use error_chain;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

use super::utils::DEPTHS;

mod error {
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct NodeIndex {
    depth: usize,
    index: usize,
}

impl NodeIndex {
    pub(crate) fn new(depth: usize, index: usize) -> Self {
        assert!(index < (1 << depth));
        Self { depth, index }
    }

    pub fn to_sibling(&self) -> Self {
        NodeIndex::new(self.depth, self.index ^ 1)
    }

    pub fn to_ancestor(&self, height: usize) -> Self {
        assert!(height <= self.depth);
        NodeIndex::new(self.depth - height, self.index >> height)
    }

    pub fn depth(&self) -> usize {
        self.depth
    }
}

pub struct FlattenLayout;
pub type FlattenCompleteTree<T> = CompleteTree<T, FlattenLayout>;

pub trait LayoutIndex {
    fn layout_index(index: &NodeIndex, total_depth: usize) -> usize;
}

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

impl LayoutIndex for FlattenLayout {
    #[inline]
    fn layout_index(tree_index: &NodeIndex, total_depth: usize) -> usize {
        assert!(tree_index.depth <= total_depth);
        (1 << tree_index.depth) + tree_index.index
    }
}

pub struct CompleteTree<T: Default + Clone, L: LayoutIndex> {
    total_depth: usize,
    data: Vec<T>,
    _phantom: PhantomData<L>,
}

impl<T: Default + Clone, L: LayoutIndex> CompleteTree<T, L> {
    pub fn new(total_depth: usize) -> Self {
        Self {
            total_depth,
            data: vec![T::default(); 2 * (1 << total_depth)],
            _phantom: PhantomData::default(),
        }
    }
}

impl<T: Default + Clone, L: LayoutIndex> IndexMut<NodeIndex> for CompleteTree<T, L> {
    fn index_mut(&mut self, tree_index: NodeIndex) -> &mut Self::Output {
        let layout_index = <L as LayoutIndex>::layout_index(&tree_index, self.total_depth);
        return &mut self.data[layout_index];
    }
}

impl<T: Default + Clone, L: LayoutIndex> Index<NodeIndex> for CompleteTree<T, L> {
    type Output = T;

    fn index(&self, tree_index: NodeIndex) -> &Self::Output {
        let layout_index = <L as LayoutIndex>::layout_index(&tree_index, self.total_depth);
        return &self.data[layout_index];
    }
}

#[test]
fn test_tree() {
    const LOCAL_DEPTH: usize = 3;
    const TMP_RATIO: usize = 719323;

    let db = crate::db::open_db("./__unit_test", 0u32);
    let mut tree = TreeAccess::<u64, FlattenLayout>::new("test".to_string(), LOCAL_DEPTH, db);

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

    ::std::fs::remove_dir_all("./__unit_test").unwrap();
}
