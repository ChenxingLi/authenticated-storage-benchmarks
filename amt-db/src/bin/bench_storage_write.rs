#![allow(dead_code, unused)]

use amt_db::storage::{open_col, KeyValueDbTrait};
// use crc32fast::Hasher;
use cfx_storage::storage_db::KeyValueDbTraitRead;
use crc64fast::Digest as Hasher;
use keccak_hash::keccak;
use pprof::protos::Message;
use rand::Rng;
use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};

fn hash(input: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.write(input);
    let checksum = hasher.sum64();
    unsafe { std::mem::transmute::<[u64; 4], [u8; 32]>([checksum; 4]) }
}

fn prefix(input: &[u8], len: usize) -> Vec<u8> {
    let mut base = [0u8].repeat(len);
    base.extend_from_slice(input);
    base
}

fn run_task(index: usize, common_prefix: usize, key_chunk: usize, value_chunk: usize) {
    let mut rng = ::rand::thread_rng();
    let db = open_col(&format!("./__benchmark_db_{}", index), 0u32);

    let key = |i: usize| {
        prefix(
            &hash(&i.to_be_bytes().repeat(key_chunk)),
            common_prefix * 32,
        )
    };
    let value = |i: usize| hash(&i.to_le_bytes()).repeat(value_chunk);

    assert_eq!(value(0).len(), value_chunk * 32);

    const BATCH_SIZE: usize = 250_000;
    const LAST_SECONDS: u64 = 20;

    let mut i = 0usize;
    let start = Instant::now();
    let mut time = Instant::now();

    loop {
        if i % BATCH_SIZE == 0 && i > 0 {
            println!(
                "Time {:.3}, {:>5} writes, {:.2} ops",
                start.elapsed().as_secs_f64(),
                i,
                BATCH_SIZE as f64 / time.elapsed().as_secs_f64()
            );
            time = Instant::now();

            if start.elapsed().as_secs() >= LAST_SECONDS {
                println!(
                    "****** Total write {:.2} ops",
                    i as f64 / start.elapsed().as_secs_f64()
                );
                break;
            }
        }

        i += 1;
        db.put(&key(i), &value(i)).unwrap();
    }

    let i_max = i;
    let mut i = 0usize;
    let start = Instant::now();
    let mut time = Instant::now();

    loop {
        if i % BATCH_SIZE == 0 && i > 0 {
            println!(
                "Time {:.3}, {:>5} reads, {:.2} ops",
                start.elapsed().as_secs_f64(),
                i,
                BATCH_SIZE as f64 / time.elapsed().as_secs_f64()
            );
            time = Instant::now();

            if start.elapsed().as_secs() >= LAST_SECONDS {
                println!(
                    "****** Total read {:.2} ops",
                    i as f64 / start.elapsed().as_secs_f64()
                );
                break;
            }
        }

        i += 1;
        let index = rng.gen_range(0, i_max);
        assert_eq!(
            value(index).as_slice(),
            db.get(&key(index)).unwrap().unwrap().as_ref()
        );
    }

    std::fs::remove_dir_all("./__benchmark_db").unwrap();
}

fn main() {
    let guard = pprof::ProfilerGuard::new(250).unwrap();

    // const KEY_CHUNK: usize = 2;
    // const VALUE_CHUNK: usize = 1;
    // const COMMON_PREFIX: usize = 0;

    let params = vec![[1usize, 1, 0], [1, 2, 0], [1, 4, 0], [2, 1, 0], [4, 1, 0]];

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
}
