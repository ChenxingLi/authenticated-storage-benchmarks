pub mod access;
pub mod layout;
#[macro_use]
pub mod serde;
pub mod kvdb;

pub use self::access::DBAccess;
pub use self::kvdb::DBColumn;
pub use self::layout::{FlattenArray, FlattenTree, LayoutTrait};
pub use self::serde::{StorageDecodable, StorageEncodable, StoreByBytes};
#[cfg(test)]
pub use self::test_tools::{test_db_col, test_kvdb};

use error_chain;

error_chain! {
    links {
        SerdeErr(self::serde::Error,self::serde::ErrorKind);
    }

    foreign_links {
        IoErr(std::io::Error);
    }
}

#[cfg(test)]
mod test_tools {
    use super::DBColumn;
    use kvdb::KeyValueDB;
    use std::sync::Arc;

    pub fn test_db_col() -> DBColumn {
        DBColumn::from_kvdb(Arc::new(kvdb_memorydb::create(1)), 0)
    }

    pub fn test_kvdb(num_cols: u32) -> Arc<dyn KeyValueDB> {
        Arc::new(kvdb_memorydb::create(num_cols))
    }
}
