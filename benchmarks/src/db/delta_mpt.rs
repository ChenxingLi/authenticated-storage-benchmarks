use crate::db::AuthDB;
use cfx_primitives::StorageKey;
use cfx_storage::{
    state::StateTrait, state_manager::StateManagerTrait, StateIndex, StorageConfiguration,
    StorageManager, StorageState,
};
use kvdb::KeyValueDB;
use primitive_types::H256;

use std::sync::Arc;

pub struct DeltaMpt {
    manager: Arc<StorageManager>,
    state: StorageState,
}

pub fn new(dir: &str) -> DeltaMpt {
    let config = StorageConfiguration::new_default(dir, 2000);
    let manager = Arc::new(StorageManager::new(config).unwrap());
    let state = manager.get_state_for_genesis_write();
    DeltaMpt { manager, state }
}

impl AuthDB for DeltaMpt {
    fn get(&self, key: Vec<u8>) -> Option<Box<[u8]>> {
        let key = StorageKey::AccountKey(key.as_slice());
        self.state.get(key).unwrap()
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let key = StorageKey::AccountKey(key.as_slice());
        self.state.set(key, value.into_boxed_slice()).unwrap()
    }

    fn commit(&mut self, index: usize) {
        let mut epoch_id = H256::default();
        epoch_id.0[0..8].copy_from_slice(index.to_le_bytes().as_ref());

        let state_root = self.state.compute_state_root().unwrap();
        self.state.commit(epoch_id).unwrap();
        let state_index = StateIndex::new_for_next_epoch(
            &epoch_id,
            &state_root,
            index as u64 + 1,
            self.manager
                .get_storage_manager()
                .get_snapshot_epoch_count(),
        );
        self.state = self
            .manager
            .get_state_for_next_epoch(state_index)
            .expect("unwrap result")
            .expect("unwrap option")
    }

    fn backend(&self) -> &dyn KeyValueDB {
        unimplemented!()
    }
}
