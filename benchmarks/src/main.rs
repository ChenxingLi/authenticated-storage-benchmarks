extern crate num;
extern crate parity_util_mem;
extern crate structopt;
#[macro_use]
extern crate strum_macros;

use fs_extra::dir::CopyOptions;
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

fn main() {
    let options: Options = Options::from_args();
    if options.stat_mem && !options.no_stat {
        panic!("Stat will introduce memory cost")
    }
    println!(
        "Testing {:?} with {:e} addresses",
        options.algorithm, options.total_keys
    );

    let db_dir = &options.db_dir;
    let _ = fs::remove_dir_all(db_dir);
    fs::create_dir_all(db_dir).unwrap();

    if let Some(ref warmup_dir) = options.warmup_from() {
        println!("warmup from {}", warmup_dir);
        // let a = get_dir_content2(warmup_dir, &DirOptions::new()).unwrap();
        // dbg!(a.files);
        // dbg!(a.directories);
        let mut options = CopyOptions::new();
        options.content_only = true;
        fs_extra::dir::copy(warmup_dir, db_dir, &options).unwrap();
    }

    if let Some(ref dir) = options.report_dir {
        fs::create_dir_all(dir).unwrap()
    }

    let tasks = tasks::ReadThenWrite::<rand_pcg::Pcg64>::new(&options);
    let (backend, backend_any) = backend::backend(&options);
    let (db, reporter) = db::new(backend, &options);
    run_tasks(db, backend_any, tasks, reporter, &options);
}
