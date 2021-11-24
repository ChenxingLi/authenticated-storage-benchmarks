use std::path::Path;
use std::sync::Arc;

use crate::opts::Options;
use cfx_kvdb_rocksdb::{CompactionProfile, Database, DatabaseConfig};

pub fn open(db_dir: &str, num_cols: u32, opts: &Options) -> Arc<Database> {
    let mut db_config = DatabaseConfig::with_columns(num_cols);

    db_config.memory_budget = Some(128);
    db_config.compaction = CompactionProfile::auto(Path::new(db_dir));
    db_config.disable_wal = false;
    db_config.enable_statistics = !opts.no_stat;

    let db = Database::open(&db_config, db_dir).unwrap();

    Arc::new(db)
}
