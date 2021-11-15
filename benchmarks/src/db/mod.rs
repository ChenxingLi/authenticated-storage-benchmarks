mod amt;
mod delta_mpt;
mod mpt;
mod raw;

use amt::AMTCounter;

use crate::run::counter::Reporter;
use crate::TestMode;

use crate::backend::BackendType;
use std::fs;

pub trait AuthDB {
    fn get(&self, key: Vec<u8>) -> Option<Box<[u8]>>;
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>);
    fn commit(&mut self, index: usize);
}

pub fn new(
    mode: TestMode,
    db_dir: &str,
    db_type: BackendType,
    report_dir: &str,
    prefix: String,
) -> (Box<dyn AuthDB>, Reporter) {
    let db: Box<dyn AuthDB> = match mode {
        TestMode::RAW => Box::new(raw::new(db_dir, db_type)),
        TestMode::AMT => Box::new(amt::new(db_dir, db_type)),
        TestMode::MPT => Box::new(mpt::new(db_dir, db_type)),
        TestMode::DMPT => Box::new(delta_mpt::new(db_dir)),
    };

    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(report_dir.to_string() + "/timing.log")
        .unwrap();

    let mut reporter = Reporter::new(file, prefix);
    if matches!(mode, TestMode::AMT) {
        reporter.set_counter::<AMTCounter>();
    }
    return (db, reporter);
}
