use crate::db::AuthDB;
use amt_db::storage::{open_col, KvdbRocksdb};

pub fn new(dir: &str) -> KvdbRocksdb {
    open_col(dir, 0u32)
}

impl AuthDB for KvdbRocksdb {
    fn get(&self, key: Vec<u8>) -> Option<Box<[u8]>> {
        self.get(key.as_slice()).unwrap()
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.put(key.as_slice(), value.as_slice()).unwrap();
    }

    fn commit(&mut self, _index: usize) {
        self.flush()
    }
}
