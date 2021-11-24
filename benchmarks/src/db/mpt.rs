use keccak_hasher::KeccakHasher;
use kvdb::{DBTransaction, KeyValueDB};
use parity_journaldb::{Algorithm, JournalDB};
use patricia_trie_ethereum::RlpCodec;
use primitive_types::H256;
use std::cell::RefCell;
use std::collections::BTreeMap;

use hash_db::Hasher;
use std::sync::Arc;
use trie_db::{Trie, TrieMut};

use crate::db::AuthDB;
use crate::run::CounterTrait;

pub type TrieDBMut<'db> = trie_db::TrieDBMut<'db, KeccakHasher, RlpCodec>;
pub type TrieDB<'db> = trie_db::TrieDB<'db, KeccakHasher, RlpCodec>;

pub struct MptDB {
    backing: Arc<dyn KeyValueDB>,
    db: Arc<RefCell<Box<dyn JournalDB>>>,
    root: H256,
}

fn epoch_hash(epoch: usize) -> H256 {
    KeccakHasher::hash(&epoch.to_le_bytes())
}

pub(crate) fn new(backend: Arc<dyn KeyValueDB>) -> MptDB {
    let db = parity_journaldb::new(backend.clone(), Algorithm::OverlayRecent, 0);
    let db = Arc::new(RefCell::new(db));

    MptDB {
        db,
        backing: backend,
        root: KECCAK_NULL_RLP,
    }
}

impl AuthDB for MptDB {
    // This logic is in function `require_or_from` of OpenEthereum
    fn get(&self, key: Vec<u8>) -> Option<Box<[u8]>> {
        let db = self.db.borrow();
        let hash_db = &db.as_hash_db();

        let trie = TrieDB::new(hash_db, &self.root).unwrap();
        trie.get(key.as_slice())
            .unwrap()
            .map(|x| x.into_vec().into_boxed_slice())
    }

    // This logic is in function `commit` in `ethcore/src/state/run` of OpenEthereum
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let mut db = self.db.borrow_mut();
        let hash_db = db.as_hash_db_mut();

        let mut trie = TrieDBMut::from_existing(hash_db, &mut self.root).unwrap();
        trie.insert(key.as_slice(), value.as_slice()).unwrap();
    }

    // This logic is in function `commit` in `ethcore/src/state/run` of OpenEthereum
    fn commit(&mut self, index: usize) {
        let mut batch = DBTransaction::new();
        let mut db = self.db.borrow_mut();

        // The third parameter is not used in archive journal db. We feed an arbitrary data.
        db.journal_under(&mut batch, index as u64, &epoch_hash(index))
            .unwrap();
        if let Some(old_index) = index.checked_sub(JOURNAL_EPOCH) {
            db.mark_canonical(&mut batch, old_index as u64, &epoch_hash(old_index))
                .unwrap();
        }
        db.backing().write(batch).unwrap();
        db.flush();
    }

    fn backend(&self) -> &dyn KeyValueDB {
        &*self.backing
    }
}

pub struct MptCounter {
    journal_db: Arc<RefCell<Box<dyn JournalDB>>>,
}

impl MptCounter {
    pub fn from_mpt_db(mpt_db: &MptDB) -> Self {
        Self {
            journal_db: mpt_db.db.clone(),
        }
    }
}

impl CounterTrait for MptCounter {
    fn report(&mut self) -> String {
        let mut sizes = BTreeMap::new();
        self.journal_db.borrow().get_sizes(&mut sizes);
        format!(
            "Recent backing size: {}",
            sizes.get("db_overlay_recent_backing_size").unwrap()
        )
    }
}

/// The KECCAK of the RLP encoding of empty data.
pub const KECCAK_NULL_RLP: H256 = H256([
    0x56, 0xe8, 0x1f, 0x17, 0x1b, 0xcc, 0x55, 0xa6, 0xff, 0x83, 0x45, 0xe6, 0x92, 0xc0, 0xf8, 0x6e,
    0x5b, 0x48, 0xe0, 0x1b, 0x99, 0x6c, 0xad, 0xc0, 0x01, 0x62, 0x2f, 0xb5, 0xe3, 0x63, 0xb4, 0x21,
]);

const JOURNAL_EPOCH: usize = 50;
