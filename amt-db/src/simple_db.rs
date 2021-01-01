use crate::crypto::{AMTParams, Pairing};
use crate::storage::{
    KeyValueDbTrait, KeyValueDbTraitRead, StorageDecodable, StorageEncodable, StoreTupleByBytes,
};
use crate::ver_tree::{Key, VerForest};
use cfx_storage::KvdbRocksdb;
use db::SystemDB;
use std::sync::Arc;

const COL_VER_TREE: u32 = 0;
const COL_KV_CHANGE: u32 = COL_VER_TREE + 1;
const COL_TREE_CHANGE: u32 = COL_KV_CHANGE + 1;
const COL_KEY_META: u32 = COL_TREE_CHANGE + 1;

const WRITE_IN_BATCH: bool = false;

struct SimpleDb {
    version_tree: VerForest,
    db_key_metadata: KvdbRocksdb,
    db_kv_changes: KvdbRocksdb,
    db_tree_changes: KvdbRocksdb,

    uncommitted_key_values: Vec<(Key, Box<[u8]>)>,
    dirty_guard: bool,
}

impl StoreTupleByBytes for (u64, u64) {}
// impl StoreTupleByBytes for (u64, usize) {}

impl SimpleDb {
    fn new(database: Arc<SystemDB>, pp: Arc<AMTParams<Pairing>>) -> Self {
        let db_ver_tree = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_VER_TREE,
        };
        let version_tree = VerForest::new(db_ver_tree, pp);
        let db_kv_changes = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_KV_CHANGE,
        };
        let db_tree_changes = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_TREE_CHANGE,
        };
        let db_key_metadata = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_KEY_META,
        };
        Self {
            version_tree,
            db_key_metadata,
            db_kv_changes,
            db_tree_changes,
            uncommitted_key_values: Vec::new(),
            dirty_guard: false,
        }
    }

    fn get(&self, key: &Key) -> Option<Box<[u8]>> {
        assert!(!self.dirty_guard);

        let (recent_epoch, position) = self
            .db_key_metadata
            .get(key.as_ref())
            .unwrap()
            .map_or(Default::default(), |x| <(u64, u64)>::storage_decode(&*x));

        if recent_epoch == 0 {
            return None;
        }

        Some(
            self.db_kv_changes
                .get(&(recent_epoch, position).storage_encode())
                .unwrap()
                .expect("Should find a key"),
        )
    }

    fn set(&mut self, key: &Key, value: Box<[u8]>) -> () {
        self.dirty_guard = true;
        self.uncommitted_key_values.push((key.clone(), value))
    }

    fn commit(&mut self, epoch: u64) {
        for (position, (key, value)) in self.uncommitted_key_values.drain(..).enumerate() {
            let position = position as u64;
            let _version = self.version_tree.inc_key(&key);
            self.db_kv_changes
                .put(&(epoch, position).storage_encode(), &value)
                .unwrap();

            self.db_key_metadata
                .put(key.as_ref(), &(epoch, position).storage_encode())
                .unwrap();

            // TODO: Make merkle tree. With (key,version,value)
        }
        let mut updated_tree_commitment = self.version_tree.commit();
        for (position, (tree, commitment, _version)) in
            updated_tree_commitment.drain(..).enumerate()
        {
            let position = position as u64;
            self.db_tree_changes
                .put(
                    &(epoch, position).storage_encode(),
                    &(tree, commitment).storage_encode(),
                )
                .unwrap();

            // TODO: Make merkle tree. With (name,version,value)
        }
        self.dirty_guard = false;
    }
}
