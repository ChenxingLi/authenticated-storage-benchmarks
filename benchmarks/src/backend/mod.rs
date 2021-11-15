#![allow(unused)]

use kvdb::KeyValueDB;
use std::path::Path;
use std::sync::Arc;

pub mod db_with_mertics;
mod in_mem_with_metrics;

pub struct BackendType;

pub fn backend(db_dir: &str, num_cols: u32, db_type: BackendType) -> Arc<dyn KeyValueDB> {
    db::open_database(
        db_dir,
        &db::db_config(
            Path::new(db_dir),
            Some(128),
            db::DatabaseCompactionProfile::default(),
            num_cols,
            false,
        ),
    )
    .map_err(|e| format!("Failed to open database {:?}", e))
    .unwrap()
    .key_value()
    .clone()
}
