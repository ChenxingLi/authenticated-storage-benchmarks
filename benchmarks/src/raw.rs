use crate::run::BenchmarkDB;
use amt_db::storage::{open_col, KvdbRocksdb};
use cfx_storage::storage_db::KeyValueDbTraitRead;
use cfx_storage::KeyValueDbTrait;

pub fn new(dir: &str) -> KvdbRocksdb {
    open_col(dir, 0u32)
}

impl BenchmarkDB for KvdbRocksdb {
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
