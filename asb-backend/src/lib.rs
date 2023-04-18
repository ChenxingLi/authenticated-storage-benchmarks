use asb_options::{Backend, Options};
use kvdb::KeyValueDB;
use std::sync::Arc;

#[cfg(feature = "cfx-backend")]
mod cfx_kvdb_rocksdb;

#[cfg(feature = "cfx-backend")]
mod db_with_mertics;

mod in_mem_with_metrics;
mod mdbx;

#[cfg(feature = "parity-backend")]
mod parity_kvdb_rocksdb;

pub fn backend(opts: &Options) -> Arc<dyn KeyValueDB> {
    match opts.backend {
        Backend::RocksDB => {
            let db_dir = opts.db_dir.as_str();
            #[cfg(feature = "cfx-backend")]
            {
                cfx_kvdb_rocksdb::open(db_dir, opts)
            }
            #[cfg(feature = "parity-backend")]
            {
                parity_kvdb_rocksdb::open(db_dir, opts.num_cols())
            }
        }
        Backend::InMemoryDB => Arc::new(kvdb_memorydb::create(opts.num_cols())),
        Backend::MDBX => Arc::new(mdbx::open_database(opts)),
    }
}
