mod amt;
#[cfg(feature = "cfx-backend")]
mod delta_mpt;
mod mpt;
mod raw;

use amt::AMTCounter;
use kvdb::KeyValueDB;
use std::sync::Arc;

use crate::db::mpt::MptCounter;
use crate::run::counter::{Counter, Reporter};
use crate::run::CounterTrait;
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
    let (db, counter): (Box<dyn AuthDB>, Box<dyn CounterTrait>) = match opts.algorithm {
        TestMode::RAW => (Box::new(raw::new(backend)), Box::new(Counter::default())),
        TestMode::AMT => (Box::new(amt::new(backend)), Box::new(AMTCounter::default())),
        TestMode::MPT => {
            let mpt_db = mpt::new(backend);
            let counter = MptCounter::from_mpt_db(&mpt_db);
            (Box::new(mpt_db), Box::new(counter))
        }
        TestMode::DMPT => (open_dmpt(&opts.db_dir), Box::new(Counter::default())),
    };

    let mut reporter = Reporter::new(opts);
    reporter.set_counter(counter);

    return (db, reporter);
}
