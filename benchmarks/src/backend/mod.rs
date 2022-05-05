use crate::opts::Options;
use kvdb::KeyValueDB;
use std::any::Any;
use std::sync::Arc;

#[cfg(feature = "cfx-backend")]
mod cfx_kvdb_rocksdb;

#[cfg(feature = "cfx-backend")]
mod db_with_mertics;

mod in_mem_with_metrics;

#[cfg(feature = "parity-backend")]
mod parity_kvdb_rocksdb;

pub fn backend(opts: &Options) -> (Arc<dyn KeyValueDB>, Arc<dyn Any>) {
    let db_dir = opts.db_dir.as_str();
    #[cfg(feature = "cfx-backend")]
    {
        let db = cfx_kvdb_rocksdb::open(db_dir, opts);
        (db.clone(), db)
    }
    // #[cfg(feature = "in-memory-backend")]
    // {
    //     let _ = db_dir;
    //     Arc::new(kvdb_memorydb::create(num_cols))
    // }
    #[cfg(feature = "parity-backend")]
    {
        parity_kvdb_rocksdb::open(db_dir, num_cols)
    }
}
