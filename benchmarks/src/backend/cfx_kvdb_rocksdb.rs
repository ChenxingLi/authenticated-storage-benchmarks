use std::path::Path;
use std::sync::Arc;

use cfx_kvdb_rocksdb::{CompactionProfile, Database, DatabaseConfig};

use crate::opts::Options;
use crate::TestMode;

pub fn open(db_dir: &str, opts: &Options) -> Arc<Database> {
    let num_cols = match opts.algorithm {
        TestMode::AMT => amt_db::amt_db::NUM_COLS,
        _ => 1,
    };
    let mut db_config = DatabaseConfig::with_columns(num_cols);

    db_config.memory_budget = Some(opts.cache_size as usize);
    db_config.compaction = CompactionProfile::auto(Path::new(db_dir));
    db_config.disable_wal = false;
    db_config.enable_statistics = !opts.no_stat;

    let db = Database::open(&db_config, db_dir).unwrap();

    Arc::new(db)
}
