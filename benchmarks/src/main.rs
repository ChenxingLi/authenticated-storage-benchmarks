extern crate num;
extern crate parity_util_mem;
extern crate structopt;
#[macro_use]
extern crate strum_macros;

use std::fs;
use structopt::StructOpt;

mod backend;
mod db;
mod opts;
mod run;
mod tasks;

use opts::{Options, TestMode};
use run::run_tasks;

// const DIR: &'static str = "/mnt/tmpfs/__benchmarks";
const DIR: &'static str = "./__benchmarks";

fn main() {
    let options: Options = Options::from_args();
    println!(
        "Testing {:?} with {:e} addresses",
        options.algorithm, options.total_keys
    );

    if let Some(ref dir) = options.report_dir {
        fs::create_dir_all(dir).unwrap()
    }

    let tasks = tasks::ReadThenWrite::<rand_pcg::Pcg64>::new(&options);
    let (db, reporter) = db::new(DIR, &options);
    run_tasks(db, tasks, reporter, &options);

    if let Some(ref dir) = options.report_dir {
        let _ = fs::remove_dir_all(dir);
    }
}
