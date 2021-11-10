pub mod access;
pub mod layout;
pub mod rocksdb;
#[macro_use]
pub mod serde;
pub mod kvdb;

pub use self::access::DBAccess;
pub use self::kvdb::DBColumn;
pub use self::layout::{FlattenArray, FlattenTree, LayoutTrait};
pub use self::rocksdb::{open_col, open_database, open_kvdb, KvdbRocksdb, SystemDB};
pub use self::serde::{StorageDecodable, StorageEncodable, StoreByBytes};

use error_chain;
error_chain! {
    links {
        SerdeErr(self::serde::Error,self::serde::ErrorKind);
        RocksDbErr(cfx_storage::Error, cfx_storage::ErrorKind);
    }

    foreign_links {
        IoErr(std::io::Error);
    }
}

// Re-export cfx_storage
pub use cfx_storage::{storage_db::KeyValueDbTraitRead, KeyValueDbTrait};
