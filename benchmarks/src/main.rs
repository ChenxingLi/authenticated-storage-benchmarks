extern crate num;
#[macro_use]
extern crate num_derive;
extern crate parity_util_mem;

use num::FromPrimitive;

mod backend;
mod db;
mod run;
mod tasks;

use crate::backend::BackendType;
use crate::run::run_tasks;
use std::{env, fs};

const SEED: u64 = 64;
const SECONDS: u64 = 3600;
const BATCH_SIZE: usize = 1_000;

const REPORT_EPOCH: usize = 50;
const PROFILE_EPOCH: usize = 5_000;
const MAX_EPOCH: usize = 100_000;

// const DIR: &'static str = "/mnt/tmpfs/__benchmarks";
const DIR: &'static str = "./__benchmarks";
const REPORT_DIR: &'static str = "./__reports";

fn main() {
    fs::create_dir_all(REPORT_DIR).unwrap();

    let args: Vec<String> = env::args().collect();
    let mode: TestMode = FromPrimitive::from_u8(args[1].parse().unwrap()).unwrap();
    let total_keys: u64 = parse_num(&args[2]);
    let settings = format!("{:?},{:e}", mode, total_keys);
    println!("Testing {:?} with {:e} addresses", mode, total_keys);

    let tasks = tasks::ReadThenWrite::<rand_pcg::Pcg64>::new(total_keys as usize, BATCH_SIZE);
    let (db, reporter) = db::new(mode, DIR, BackendType, REPORT_DIR, settings);
    run_tasks(db, tasks, reporter);

    let _ = std::fs::remove_dir_all(DIR);
}

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
