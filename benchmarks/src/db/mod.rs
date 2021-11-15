mod amt;
mod delta_mpt;
mod mpt;
mod raw;

use amt::AMTCounter;

use crate::backend::BackendType;
use crate::run::counter::Reporter;
use crate::{Options, TestMode};

pub trait AuthDB {
    fn get(&self, key: Vec<u8>) -> Option<Box<[u8]>>;
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>);
    fn commit(&mut self, index: usize);
}

pub fn new<'a>(db_dir: &str, opts: &'a Options) -> (Box<dyn AuthDB>, Reporter<'a>) {
    let db_type = BackendType;
    let db: Box<dyn AuthDB> = match opts.algorithm {
        TestMode::RAW => Box::new(raw::new(db_dir, db_type)),
        TestMode::AMT => Box::new(amt::new(db_dir, db_type)),
        TestMode::MPT => Box::new(mpt::new(db_dir, db_type)),
        TestMode::DMPT => Box::new(delta_mpt::new(db_dir)),
    };

    let mut reporter = Reporter::new(opts);
    if matches!(opts.algorithm, TestMode::AMT) {
        reporter.set_counter::<AMTCounter>();
    }
    return (db, reporter);
}
