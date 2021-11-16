use crate::db::AuthDB;
use kvdb::{DBOp, DBTransaction, KeyValueDB};

use std::sync::Arc;

pub fn new(backend: Arc<dyn KeyValueDB>) -> Arc<dyn KeyValueDB> {
    backend
}

impl AuthDB for Arc<dyn KeyValueDB> {
    fn get(&self, key: Vec<u8>) -> Option<Box<[u8]>> {
        KeyValueDB::get(&**self, 0, key.as_ref())
            .unwrap()
            .map(|x| x.into_boxed_slice())
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.write_buffered(DBTransaction {
            ops: vec![DBOp::Insert {
                col: 0,
                key: key.into(),
                value,
            }],
        });
    }

    fn commit(&mut self, _index: usize) {
        self.flush().unwrap()
    }

    fn backend(&self) -> &dyn KeyValueDB {
        &**self
    }
}
