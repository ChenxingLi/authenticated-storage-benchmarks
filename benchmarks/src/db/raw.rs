use crate::db::AuthDB;
use amt_db::storage::{open_col, KvdbRocksdb};
use kvdb::{DBOp, DBTransaction};

pub fn new(dir: &str) -> KvdbRocksdb {
    open_col(dir, 0u32)
}

impl AuthDB for KvdbRocksdb {
    fn get(&self, key: Vec<u8>) -> Option<Box<[u8]>> {
        self.kvdb
            .get(0, key.as_ref())
            .unwrap()
            .map(|x| x.into_boxed_slice())
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.kvdb.write_buffered(DBTransaction {
            ops: vec![DBOp::Insert {
                col: self.col,
                key: key.into(),
                value,
            }],
        });
    }

    fn commit(&mut self, _index: usize) {
        self.kvdb.flush().unwrap()
    }
}
