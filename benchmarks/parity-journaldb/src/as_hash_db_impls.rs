// Copyright 2015-2020 Parity Technologies (UK) Ltd.
// This file is part of OpenEthereum.

// OpenEthereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// OpenEthereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with OpenEthereum.  If not, see <http://www.gnu.org/licenses/>.

//! Impls of the `AsHashDB` upcast trait for all different variants of DB
use crate::{AsKeyedHashDB, KeyedHashDB};
use archivedb::ArchiveDB;
use earlymergedb::EarlyMergeDB;
use ethereum_types::H256;
use hash_db::{AsHashDB, HashDB};
use hash_db15::{AsHashDB as AsHashDB15, HashDB as HashDB15, Prefix};
use keccak_hasher::KeccakHasher;
use keccak_hasher15::KeccakHasher as KeccakHasher15;
use kvdb::DBValue as KVDBValue;
use overlaydb::OverlayDB;
use overlayrecentdb::OverlayRecentDB;
use refcounteddb::RefCountedDB;
use trie_db::DBValue;

macro_rules! wrap_hash_db {
    ($name: ty) => {
        impl HashDB<KeccakHasher, DBValue> for $name {
            fn get(&self, key: &H256) -> Option<DBValue> {
                HashDB::<KeccakHasher, KVDBValue>::get(self, key).map(|x| DBValue::from_vec(x))
            }

            fn contains(&self, key: &H256) -> bool {
                HashDB::<KeccakHasher, KVDBValue>::contains(self, key)
            }

            fn insert(&mut self, value: &[u8]) -> H256 {
                HashDB::<KeccakHasher, KVDBValue>::insert(self, value)
            }

            fn emplace(&mut self, key: H256, value: DBValue) {
                HashDB::<KeccakHasher, KVDBValue>::emplace(self, key, value.into_vec())
            }

            fn remove(&mut self, key: &H256) {
                HashDB::<KeccakHasher, KVDBValue>::remove(self, key)
            }
        }

        impl AsHashDB<KeccakHasher, DBValue> for $name {
            fn as_hash_db(&self) -> &dyn HashDB<KeccakHasher, DBValue> {
                self
            }
            fn as_hash_db_mut(&mut self) -> &mut dyn HashDB<KeccakHasher, DBValue> {
                self
            }
        }

        impl AsHashDB<KeccakHasher, KVDBValue> for $name {
            fn as_hash_db(&self) -> &dyn HashDB<KeccakHasher, KVDBValue> {
                self
            }
            fn as_hash_db_mut(&mut self) -> &mut dyn HashDB<KeccakHasher, KVDBValue> {
                self
            }
        }

        impl AsKeyedHashDB for $name {
            fn as_keyed_hash_db(&self) -> &dyn KeyedHashDB {
                self
            }
        }

        impl AsHashDB15<KeccakHasher15, DBValue> for $name {
            fn as_hash_db(&self) -> &dyn HashDB15<KeccakHasher15, DBValue> {
                self
            }

            fn as_hash_db_mut(&mut self) -> &mut dyn HashDB15<KeccakHasher15, DBValue> {
                self
            }
        }

        impl HashDB15<KeccakHasher15, DBValue> for $name {
            // The key function `HashKey` in `memory-db` (v0.28.0) omits `prefix`.
            // The example code in `TrieDB` uses `HashKey` as key function.
            // So here we also omit `prefix`.
            fn get(&self, key: &[u8; 32], _prefix: Prefix) -> Option<DBValue> {
                HashDB::<KeccakHasher, DBValue>::get(self, to_h256_ref(key))
            }

            fn contains(&self, key: &[u8; 32], _prefix: Prefix) -> bool {
                HashDB::<KeccakHasher, DBValue>::contains(self, to_h256_ref(key))
            }

            fn insert(&mut self, _prefix: Prefix, value: &[u8]) -> [u8; 32] {
                HashDB::<KeccakHasher, DBValue>::insert(self, value).into()
            }

            fn emplace(&mut self, key: [u8; 32], _prefix: Prefix, value: DBValue) {
                HashDB::<KeccakHasher, DBValue>::emplace(self, key.into(), value)
            }

            fn remove(&mut self, key: &[u8; 32], _prefix: Prefix) {
                HashDB::<KeccakHasher, DBValue>::remove(self, to_h256_ref(key))
            }
        }
    };
}

wrap_hash_db!(ArchiveDB);
wrap_hash_db!(EarlyMergeDB);
wrap_hash_db!(OverlayRecentDB);
wrap_hash_db!(RefCountedDB);
wrap_hash_db!(OverlayDB);

fn to_h256_ref(input: &[u8; 32]) -> &H256 {
    unsafe { std::mem::transmute(input) }
}
