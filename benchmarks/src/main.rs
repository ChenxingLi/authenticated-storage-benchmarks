#![allow(unused, dead_code)]

pub mod amt;
mod db_with_mertics;
mod delta_mpt;
mod in_mem_with_metrics;
mod mpt;
pub mod raw;
pub mod run;
pub mod task_producer;

use crate::run::{BenchmarkDB, Counter, CounterTrait};
use crate::{amt::AMTCounter, run::run_tasks, task_producer::ReadThenWrite};
use pprof::protos::Message;
use rand_pcg::Pcg64;
use std::{env, fs, io::Write};

extern crate num;
#[macro_use]
extern crate num_derive;

use num::FromPrimitive;
use std::fs::File;

const SEED: u64 = 64;
const SECONDS: u64 = 3600;
const BATCH_SIZE: usize = 1_000;

const REPORT_EPOCH: usize = 50;
const PROFILE_EPOCH: usize = 5_000;
const MAX_EPOCH: usize = 100_000;

#[derive(Debug, FromPrimitive)]
pub enum TestMode {
    RAW = 1,
    AMT = 2,
    MPT = 3,
    DMPT = 4,
}

fn parse_num(s: &String) -> u64 {
    let base = match s.chars().rev().next().unwrap() {
        'k' | 'K' => 1_000,
        'm' | 'M' => 1_000_000,
        'g' | 'G' => 1_000_000_000,
        _ => 1,
    };
    let num = if base > 1 {
        let mut chars = s.chars();
        chars.next_back();
        chars.as_str()
    } else {
        s.as_str()
    };
    base * num.parse::<u64>().unwrap()
}

// const DIR: &'static str = "/mnt/tmpfs/__benchmarks";
const DIR: &'static str = "./__benchmarks";
const REPORT_DIR: &'static str = "./__reports";

fn main() {
    fs::create_dir_all(REPORT_DIR).unwrap();
    let args: Vec<String> = env::args().collect();

    let mode: TestMode = FromPrimitive::from_u8(args[1].parse().unwrap()).unwrap();
    let total_keys: u64 = parse_num(&args[2]);

    println!("Testing {:?} with {:e} addresses", mode, total_keys);

    let mut task_producer = ReadThenWrite::<Pcg64>::new(total_keys as usize, BATCH_SIZE);

    let (mut db, mut counter): (Box<dyn BenchmarkDB>, Box<dyn CounterTrait>) = match mode {
        TestMode::RAW => (Box::new(raw::new(DIR)), Box::new(Counter::new())),
        TestMode::AMT => (Box::new(amt::new(DIR)), Box::new(AMTCounter::new())),
        TestMode::MPT => (Box::new(mpt::new(DIR)), Box::new(Counter::new())),
        TestMode::DMPT => (Box::new(delta_mpt::new(DIR)), Box::new(Counter::new())),
    };

    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(REPORT_DIR.to_string() + "/timing.log")
        .unwrap();
    let prefix = format!("{:?},{:e}", mode, total_keys);

    run_tasks(
        db.as_mut(),
        &mut task_producer,
        counter.as_mut(),
        file,
        prefix,
    );

    std::fs::remove_dir_all(DIR).unwrap();
}
