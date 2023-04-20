use std::sync::{Arc, RwLock};

use authdb::AuthDB;
use kvdb::KeyValueDB;
use rainblock_trie::MerklePatriciaTree;

pub struct RainMpt(RwLock<MerklePatriciaTree<6>>);

pub fn new(backend: Arc<dyn KeyValueDB>) -> RainMpt {
    RainMpt(RwLock::new(MerklePatriciaTree::<6>::new(backend)))
}

impl AuthDB for RainMpt {
    fn get(&self, key: Vec<u8>) -> Option<Box<[u8]>> {
        self.0.write().unwrap().get(key).map(Vec::into_boxed_slice)
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.0.write().unwrap().put(key, value);
    }

    fn commit(&mut self, _index: usize) {
        self.0.write().unwrap().commit().unwrap();
    }

    fn backend(&self) -> &dyn KeyValueDB {
        unimplemented!()
    }

    fn flush_all(&mut self) {
        self.0.write().unwrap().flush_all().unwrap()
    }
}
