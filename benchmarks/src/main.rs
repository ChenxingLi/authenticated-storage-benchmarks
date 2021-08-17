pub mod amt;
pub mod raw;
pub mod run;
pub mod task_producer;

use crate::run::{BenchmarkDB, Counter, CounterTrait};
use crate::{amt::AMTCounter, run::run_tasks, task_producer::ReadThenWrite};
use pprof::protos::Message;
use rand_pcg::Pcg64;
use std::{fs::File, io::Write};

const SEED: u64 = 64;
const SECONDS: u64 = 120;
const TOTAL_KEYS: usize = 1_000_000;
const BATCH_SIZE: usize = 1_000;
const MODE: TestMode = TestMode::AMT;

pub enum TestMode {
    RAW,
    AMT,
}

fn main() {
    let mut task_producer = ReadThenWrite::<Pcg64>::new(TOTAL_KEYS, BATCH_SIZE);

    let (mut db, mut counter): (Box<dyn BenchmarkDB>, Box<dyn CounterTrait>) = match MODE {
        TestMode::RAW => (
            Box::new(raw::new("./__benchmarks")),
            Box::new(Counter::new()),
        ),
        TestMode::AMT => (
            Box::new(amt::new("./__benchmarks")),
            Box::new(AMTCounter::new()),
        ),
    };

    let guard = pprof::ProfilerGuard::new(250).unwrap();

    run_tasks(db.as_mut(), &mut task_producer, counter.as_mut(), SECONDS);

    match guard.report().build() {
        Ok(report) => {
            let mut file = File::create("./profile/profile.pb").unwrap();
            let profile = report.pprof().unwrap();

            let mut content = Vec::new();
            profile.encode(&mut content).unwrap();
            file.write_all(&content).unwrap();
        }
        Err(_) => {}
    };
    std::fs::remove_dir_all("./__benchmarks").unwrap();
}
