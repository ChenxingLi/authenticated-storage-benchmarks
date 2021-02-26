use crate::crypto::{AMTParams, Pairing};
use crate::merkle::StaticMerkleTree;
use crate::storage::{
    KeyValueDbTrait, KeyValueDbTraitRead, KvdbRocksdb, Result, StorageDecodable, StorageEncodable,
    StoreByBytes, StoreTupleByBytes, SystemDB,
};
use crate::ver_tree::{Commitment, Key, TreeName, VerForest};
use algebra::bls12_381::G1Projective;
use keccak_hash::keccak;
use std::sync::Arc;

const COL_VER_TREE: u32 = 0;
const COL_KEY_POS: u32 = COL_VER_TREE + 1;
const COL_TREE_POS: u32 = COL_KEY_POS + 1;
const COL_POS_VALUE: u32 = COL_TREE_POS + 1;
const COL_POS_VALUE_MERKLE: u32 = COL_POS_VALUE + 1;
const NUM_COLS: u32 = COL_POS_VALUE_MERKLE + 1;

const WRITE_IN_BATCH: bool = false;

struct SimpleDb {
    version_tree: VerForest,
    db_key_pos: KvdbRocksdb,
    db_tree_pos: KvdbRocksdb,
    db_pos_value: KvdbRocksdb,
    db_pos_value_merkle: KvdbRocksdb,

    uncommitted_key_values: Vec<(Key, Box<[u8]>)>,
    dirty_guard: bool,
}

/// I can not implement `impl StoreByBytes for Commitment {}` here. So I implement for
/// `G1Projective` instead.
/// See https://github.com/arkworks-rs/algebra/issues/185 for more details.  
impl StoreByBytes for G1Projective {}
impl StoreTupleByBytes for (u64, u64) {}
impl StoreTupleByBytes for (Vec<u8>, u64, u8, u8, Vec<u8>) {}
impl StoreTupleByBytes for (TreeName, u64, Commitment) {}

impl SimpleDb {
    fn new(database: Arc<SystemDB>, pp: Arc<AMTParams<Pairing>>) -> Self {
        let db_ver_tree = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_VER_TREE,
        };
        let version_tree = VerForest::new(db_ver_tree, pp);
        let db_key_pos = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_KEY_POS,
        };
        let db_tree_current = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_TREE_POS,
        };
        let db_pos_value = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_POS_VALUE,
        };
        let db_pos_value_merkle = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_POS_VALUE_MERKLE,
        };
        Self {
            version_tree,
            db_key_pos,
            db_pos_value,
            db_pos_value_merkle,
            db_tree_pos: db_tree_current,
            uncommitted_key_values: Vec::new(),
            dirty_guard: false,
        }
    }

    fn get(&self, key: &Key) -> Result<Option<Box<[u8]>>> {
        assert!(
            !self.dirty_guard,
            "Can not read db if set operations have not been committed."
        );

        let maybe_pos = self.db_key_pos.get(key.as_ref())?;
        if let Some(pos) = maybe_pos {
            return Ok(Some(
                self.db_pos_value.get(&pos)?.expect("Should find a key"),
            ));
        }
        Ok(None)
    }

    fn set(&mut self, key: &Key, value: Box<[u8]>) {
        self.dirty_guard = true;
        self.uncommitted_key_values.push((key.clone(), value))
    }

    fn commit(&mut self, epoch: u64) -> Result<()> {
        let kv_num = self.uncommitted_key_values.len();
        let mut hashes = Vec::with_capacity(kv_num);

        for (position, (key, value)) in self.uncommitted_key_values.drain(..).enumerate() {
            let position = position as u64;
            let version = self.version_tree.inc_key(&key);

            self.db_key_pos
                .put(key.as_ref(), &(epoch, position).storage_encode())?;

            self.db_pos_value
                .put(&(epoch, position).storage_encode(), &value)?;

            let key_ver_value_hash = keccak(
                &(
                    key.0,
                    version.version,
                    version.level,
                    version.slot_index,
                    value.to_vec(),
                )
                    .storage_encode(),
            );
            hashes.push(key_ver_value_hash);
        }

        for (position, (tree, commitment, version)) in
            self.version_tree.commit().drain(..).enumerate()
        {
            let position = (kv_num + position) as u64;

            self.db_tree_pos
                .put(&tree.storage_encode(), &(epoch, position).storage_encode())?;

            self.db_pos_value.put(
                &(epoch, position).storage_encode(),
                &commitment.storage_encode(),
            )?;

            let name_ver_value_hash = keccak(&(tree, version, commitment).storage_encode());
            hashes.push(name_ver_value_hash);
        }

        StaticMerkleTree::dump(self.db_pos_value_merkle.clone(), epoch, hashes);

        self.dirty_guard = false;

        Ok(())
    }

    #[allow(unused_variables, unused_mut)]
    fn prove(&mut self, key: &Key) -> Result<()> {
        let pos = self
            .db_key_pos
            .get(key.as_ref())?
            .expect("TODO: impl non-existence proof later");
        let version = self.version_tree.get_key(&key);
        let (epoch, position) = <(u64, u64)>::storage_decode(&pos).unwrap();
        let value = self.db_pos_value.get(&pos)?.expect("Should find a key");

        let key_ver_value_hash = keccak(
            &(
                key.0.clone(),
                version.version,
                version.slot_index,
                version.level,
                value.to_vec(),
            )
                .storage_encode(),
        );

        // TODO: show db tree data.

        Ok(())
    }
}

#[cfg(test)]
fn new_test_simple_db(dir: &str) -> SimpleDb {
    use crate::crypto::{TypeDepths, TypeUInt, PP};
    use crate::storage::open_database;
    const DEPTHS: usize = TypeDepths::USIZE;

    let db = open_database(dir, NUM_COLS);
    let pp = PP::<Pairing>::from_file_or_new("./pp", DEPTHS);
    let amt_param = Arc::new(AMTParams::<Pairing>::from_pp(pp, DEPTHS));

    SimpleDb::new(db, amt_param)
}

#[test]
fn test_simple_db() {
    // use crate::enable_log::enable_debug_log;
    // enable_debug_log();

    let mut db = new_test_simple_db("./__test_simple_db");

    for i in 1..=255 {
        db.set(&Key(vec![1, 2, i, 0]), vec![1, 2, i].into());
        db.commit(i as u64).unwrap();
    }

    for i in 1..=20 {
        assert_eq!(
            vec![1, 2, i],
            db.get(&Key(vec![1, 2, i, 0])).unwrap().unwrap().into_vec()
        );
    }

    std::fs::remove_dir_all("./__test_simple_db").unwrap();
}
