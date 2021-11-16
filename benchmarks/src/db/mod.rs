mod amt;
#[cfg(feature = "cfx-backend")]
mod delta_mpt;
mod mpt;
mod raw;

use amt::AMTCounter;
use kvdb::KeyValueDB;
use std::sync::Arc;

use crate::run::counter::Reporter;
use crate::{Options, TestMode};

pub trait AuthDB {
    fn get(&self, key: Vec<u8>) -> Option<Box<[u8]>>;
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>);
    fn commit(&mut self, index: usize);

    fn backend(&self) -> &dyn KeyValueDB;
}

fn open_dmpt(dir: &str) -> Box<dyn AuthDB> {
    #[cfg(feature = "cfx-backend")]
    {
        Box::new(delta_mpt::new(dir))
    }
    #[cfg(not(feature = "cfx-backend"))]
    {
        let _ = dir;
        panic!("Delta MPT can only work with feature cfx-backend!")
    }
}

pub fn new<'a>(backend: Arc<dyn KeyValueDB>, opts: &'a Options) -> (Box<dyn AuthDB>, Reporter<'a>) {
    let db: Box<dyn AuthDB> = match opts.algorithm {
        TestMode::RAW => Box::new(raw::new(backend)),
        TestMode::AMT => Box::new(amt::new(backend)),
        TestMode::MPT => Box::new(mpt::new(backend)),
        TestMode::DMPT => open_dmpt(&opts.db_dir),
    };

    let mut reporter = Reporter::new(opts);
    if matches!(opts.algorithm, TestMode::AMT) {
        reporter.set_counter::<AMTCounter>();
    }
    return (db, reporter);
}
