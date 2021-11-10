#![allow(unused)]
pub use db::SystemDB;
use std::path::Path;
use std::sync::{Arc, RwLock};

const NUM_COLUMNS: u32 = 1;

pub fn open_database(db_dir: &str, num_cols: u32) -> Arc<SystemDB> {
    db::open_database(
        db_dir,
        &db::db_config(
            Path::new(db_dir),
            Some(128),
            db::DatabaseCompactionProfile::default(),
            num_cols,
            false,
        ),
    )
    .map_err(|e| format!("Failed to open database {:?}", e))
    .unwrap()
}

pub fn open_col(db_dir: &str, col: u32) -> KvdbRocksdb {
    cfx_storage::KvdbRocksdb {
        kvdb: open_database(db_dir, NUM_COLUMNS).key_value().clone(),
        col,
    }
    .into()
}

pub fn open_kvdb(db_dir: &str, col: u32) -> Arc<dyn KeyValueDB> {
    let kvdb = open_col(db_dir, col);
    kvdb.kvdb
}

pub use cfx_storage::KvdbRocksdb;

// // Inplement batch write
// // TODO: refactor code.
// type TransactionType = <cfx_storage::KvdbRocksdb as KeyValueDbTraitTransactional>::TransactionType;
// #[derive(Clone)]
// pub struct KvdbRocksdbWithCache {
//     db: cfx_storage::KvdbRocksdb,
//     transactions: Arc<RwLock<TransactionType>>,
// }
//
// impl From<cfx_storage::KvdbRocksdb> for KvdbRocksdbWithCache {
//     fn from(db: cfx_storage::KvdbRocksdb) -> Self {
//         let transaction = db.start_transaction(false).unwrap();
//         KvdbRocksdb {
//             db,
//             transactions: Arc::new(RwLock::new(transaction)),
//         }
//     }
// }
//
// impl KvdbRocksdbWithCache {
//     pub fn get(&self, key: &[u8]) -> cfx_storage::Result<Option<Box<[u8]>>> {
//         self.db.get(key)
//     }
//
//     pub fn put(&self, key: &[u8], value: &[u8]) -> cfx_storage::Result<Option<Option<Box<[u8]>>>> {
//         self.transactions.write().unwrap().put(key, value)
//     }
//
//     pub fn flush(&self) {
//         self.transactions.write().unwrap().commit(&self.db).unwrap();
//     }
// }
// pub use KvdbRocksdbWithCache as KvdbRocksdb;

use cfx_storage::storage_db::{
    KeyValueDbTrait, KeyValueDbTraitMultiReader, KeyValueDbTraitRead, KeyValueDbTraitSingleWriter,
    KeyValueDbTraitTransactional, KeyValueDbTraitTransactionalDyn, KeyValueDbTransactionTrait,
    KeyValueDbTypes,
};
use hashbrown::HashMap;
use kvdb::KeyValueDB;

#[test]
fn test() {
    let db = open_col("./__db", 0);
    db.put(&vec![0u8], &vec![1u8, 2u8, 4u8]).unwrap();
    println!("{:?}", db.get(&vec![0u8]).unwrap().unwrap());
    std::fs::remove_dir_all("./__db").unwrap();
}
