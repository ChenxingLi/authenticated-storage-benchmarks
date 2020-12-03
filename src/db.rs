use cfx_storage::storage_db::{KeyValueDbTrait, KeyValueDbTraitRead};
use cfx_storage::KvdbRocksdb;
use db;
use db::SystemDB;
use std::path::Path;
use std::sync::Arc;

const NUM_COLUMNS: u32 = 1;

fn open_database(db_dir: &str) -> Arc<SystemDB> {
    db::open_database(
        db_dir,
        &db::db_config(
            Path::new(db_dir),
            Some(128),
            db::DatabaseCompactionProfile::default(),
            NUM_COLUMNS,
            false,
        ),
    )
    .map_err(|e| format!("Failed to open database {:?}", e))
    .unwrap()
}

fn open_db(db_dir: &str, col: u32) -> KvdbRocksdb {
    KvdbRocksdb {
        kvdb: open_database(db_dir).key_value().clone(),
        col,
    }
}

#[test]
fn test() {
    let db = open_db("./db", 0);
    db.put(&vec![0u8], &vec![1u8, 2u8, 4u8]).unwrap();
    println!("{:?}", db.get(&vec![0u8]).unwrap().unwrap());
}
