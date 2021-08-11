#![allow(unused_imports, dead_code, unused_variables)]
use amt_db::{
    crypto::TypeUInt,
    simple_db::{new_simple_db, SimpleDb, INC_KEY_COUNT, INC_KEY_LEVEL_SUM, INC_TREE_COUNT},
    storage::{access::PUT_COUNT, Result},
    type_uint,
    ver_tree::Key,
};
use cfx_types::{H256, U256};
use keccak_hash::keccak;
use pprof::{self, protos::Message, ProfilerGuard};
use rand::{prelude::*, RngCore};
use rand_pcg::Pcg64;
use std::fs::File;
use std::io::Write;
use std::ops::Sub;
use std::time::{Duration, Instant};

const DEPTHS: usize = 16;
type_uint! {
    pub struct BenchDepths(DEPTHS);
}

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
    last_stat: Option<Statistic>,
}

pub struct Statistic {
    display: Instant,
    put_count: [u64; 4],
    inc_key_count: u64,
    inc_tree_count: u64,
    inc_key_level_count: u64,
}

impl Statistic {
    fn now() -> Self {
        return Self {
            display: Instant::now(),
            put_count: *PUT_COUNT.lock().unwrap(),
            inc_key_count: *INC_KEY_COUNT.lock().unwrap(),
            inc_tree_count: *INC_TREE_COUNT.lock().unwrap(),
            inc_key_level_count: *INC_KEY_LEVEL_SUM.lock().unwrap(),
        };
    }

    fn delta(&self, other: &Self, writes: usize) -> String {
        let key_diff = self.inc_key_count - other.inc_key_count;
        let tree_diff = self.inc_tree_count - other.inc_tree_count;
        let level_diff = self.inc_key_level_count - other.inc_key_level_count;
        let avg_level = (level_diff as f64) / (key_diff as f64);

        let last = self.display.checked_duration_since(other.display).unwrap();

        format!(
            "Time {:.3?}, {:.0} ops, {:.2} us, avg levels: {:.3}, access writes {:?}, data writes {} {}",
            last,
            writes as f64 / last.as_secs_f64(),
            last.as_secs_f64()/writes as f64 * 1_000_000f64,
            avg_level,
            self.put_count
                .iter()
                .zip(other.put_count.iter())
                .map(|(x, y)| x - y)
                .collect::<Vec<u64>>(),
            key_diff * 2,
            tree_diff * 2,
        )
    }
}

impl<R: Rng> Iterator for TimeProducer<R> {
    type Item = EpochEvents;

    fn next(&mut self) -> Option<Self::Item> {
        const STEP: usize = 10;
        if self.count == 0 {
            self.last_stat = Some(Statistic::now());
        } else if self.count % STEP == 0 {
            let new_stat = Statistic::now();
            println!(
                "Time {:.2?}: Epoch {:>5}, {}",
                self.start_time.elapsed(),
                self.count,
                new_stat.delta(self.last_stat.as_ref().unwrap(), STEP * self.write_size)
            );
            self.last_stat = Some(new_stat);
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
            last_stat: None,
        }
    }
}

fn main() {
    let (mut db, _) = new_simple_db::<BenchDepths>("./__benchmark_simple_db", true);
    let tasks = TimeProducer::<Pcg64>::new((0, 2000), 120, 64);
    let guard = pprof::ProfilerGuard::new(250).unwrap();
    let no_opt_answer = run_tasks(&mut db, tasks).expect("no db error");

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

    println!("Don't optimization answer {}", no_opt_answer);
    std::fs::remove_dir_all("./__benchmark_simple_db").unwrap();
    println!("Hello world");
}
