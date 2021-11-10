use keccak_hasher::KeccakHasher;
use kvdb::DBTransaction;
use parity_journaldb::{Algorithm, JournalDB};
use patricia_trie_ethereum::RlpCodec;
use primitive_types::H256;
use std::sync::Arc;
use trie_db::{Trie, TrieMut};

use crate::backend::db_with_mertics::DatabaseWithMetrics;
use crate::db::AuthDB;
use amt_db::storage::open_database;

pub type TrieDBMut<'db> = trie_db::TrieDBMut<'db, KeccakHasher, RlpCodec>;
pub type TrieDB<'db> = trie_db::TrieDB<'db, KeccakHasher, RlpCodec>;

pub struct MptDB {
    db: Box<dyn JournalDB>,
    root: H256,
}

pub(crate) fn new(dir: &str) -> MptDB {
    let backend_db = open_database(dir, 1).key_value().clone();
    let backing = Arc::new(DatabaseWithMetrics::new(backend_db));
    // let backing = Arc::new(InMemoryWithMetrics::create(1));
    let db = parity_journaldb::new(backing.clone(), Algorithm::Archive, 0);

    MptDB {
        db,
        root: KECCAK_NULL_RLP,
    }
}

impl AuthDB for MptDB {
    // This logic is in function `require_or_from` of OpenEthereum
    fn get(&self, key: Vec<u8>) -> Option<Box<[u8]>> {
        let db = &self.db.as_hash_db();
        let trie = TrieDB::new(db, &self.root).unwrap();
        trie.get(key.as_slice())
            .unwrap()
            .map(|x| x.into_vec().into_boxed_slice())
    }

    // This logic is in function `commit` in `ethcore/src/state/run` of OpenEthereum
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let mut trie = TrieDBMut::from_existing(self.db.as_hash_db_mut(), &mut self.root).unwrap();
        trie.insert(key.as_slice(), value.as_slice()).unwrap();
    }

    // This logic is in function `commit` in `ethcore/src/state/run` of OpenEthereum
    fn commit(&mut self, index: usize) {
        let mut batch = DBTransaction::new();
        // The third parameter is not used in archive journal db. We feed an arbitrary data.
        self.db
            .journal_under(&mut batch, index as u64, &H256::default())
            .unwrap();
        self.db.backing().write(batch).unwrap();
        self.db.flush();
    }
}

/// The KECCAK of the RLP encoding of empty data.
pub const KECCAK_NULL_RLP: H256 = H256([
    0x56, 0xe8, 0x1f, 0x17, 0x1b, 0xcc, 0x55, 0xa6, 0xff, 0x83, 0x45, 0xe6, 0x92, 0xc0, 0xf8, 0x6e,
    0x5b, 0x48, 0xe0, 0x1b, 0x99, 0x6c, 0xad, 0xc0, 0x01, 0x62, 0x2f, 0xb5, 0xe3, 0x63, 0xb4, 0x21,
]);
