#![allow(unused_imports, dead_code, unused_variables)]
use amt_db::crypto::TypeDepths;
use amt_db::{
    simple_db::{new_simple_db, SimpleDb},
    storage::Result,
    ver_tree::Key,
};
use cfx_types::{H256, U256};
use keccak_hash::keccak;
use pprof;
use pprof::protos::Message;
use pprof::ProfilerGuard;
use rand::prelude::*;
use rand::RngCore;
use rand_pcg::Pcg64;
use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};

type Value = Vec<u8>;

pub struct EpochEvents {
    reads: Vec<Key>,
    writes: Vec<(Key, Value)>,
}

pub fn run_tasks(db: &mut SimpleDb, epochs: impl Iterator<Item = EpochEvents>) -> Result<u8> {
    let mut prevent_opt_data = 0u8;
    for (epoch, events) in epochs.enumerate() {
        for key in events.reads {
            let value = db.get(&key)?;
            prevent_opt_data ^= value.and_then(|x| x.first().cloned()).unwrap_or(0);
        }
        for (key, value) in events.writes {
            db.set(&key, value.into());
        }
        let _ = db.commit(epoch as u64)?;
    }
    Ok(prevent_opt_data)
}

pub struct TimeProducer<R: Rng> {
    start_time: Instant,
    total_seconds: Duration,
    read_size: usize,
    write_size: usize,
    count: usize,
    random: R,
    last_display: Instant,
}

impl<R: Rng> Iterator for TimeProducer<R> {
    type Item = EpochEvents;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count % 100 == 0 {
            println!(
                "Time {:?}: Epoch {} (last display {:?})",
                self.start_time.elapsed(),
                self.count,
                self.last_display.elapsed(),
            );
            self.last_display = Instant::now();

            match self.guard.report().build() {
                Ok(report) => {
                    let mut file =
                        File::create(format!("./profile/epoch_{}.pb", self.count)).unwrap();
                    let profile = report.pprof().unwrap();

                    let mut content = Vec::new();
                    profile.encode(&mut content).unwrap();
                    file.write_all(&content).unwrap();
                }
                Err(_) => {}
            };
        }

        if self.start_time.elapsed() > self.total_seconds {
            return None;
        }
        let max_index = self.count * self.write_size;
        let reads = (0..self.read_size)
            .map(|_| self.random.gen_range(0, max_index + 1))
            .map(|x| Key(keccak(x.to_be_bytes()).0.to_vec()))
            .collect();
        let writes = (self.count * self.write_size..(self.count + 1) * self.write_size)
            .map(|x| {
                (
                    Key(keccak(x.to_be_bytes()).0.to_vec()),
                    self.random.gen::<[u8; 32]>().to_vec(),
                )
            })
            .collect();

        self.count += 1;
        Some(EpochEvents { reads, writes })
    }
}

impl<R: Rng + SeedableRng> TimeProducer<R> {
    fn new((read_size, write_size): (usize, usize), seconds: u64, seed: u64) -> Self {
        TimeProducer {
            start_time: Instant::now(),
            total_seconds: Duration::new(seconds, 0),
            read_size,
            write_size,
            count: 0,
            random: SeedableRng::seed_from_u64(seed),
            last_display: Instant::now(),
        }
    }
}

fn main() {
    let mut db = new_simple_db::<TypeDepths>("./__benchmark_db", true);
    let tasks = TimeProducer::<Pcg64>::new((20, 20), 18, 64);
    let guard = pprof::ProfilerGuard::new(100).unwrap();
    let no_opt_answer = run_tasks(&mut db, tasks).expect("no db error");

    println!("No optimization answer {}", no_opt_answer);
    std::fs::remove_dir_all("./__benchmark_db").unwrap();
    println!("Hello world");
}
