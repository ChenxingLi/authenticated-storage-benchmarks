use super::Profiler;
use crate::db::AuthDB;
use crate::Options;

use kvdb::IoStatsKind;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

use num_format::{Locale, WriteFormatted};

pub struct Reporter<'a> {
    pub start_time: Instant,
    total_count: usize,
    log_file: Option<File>,

    round_start_time: Instant,
    round_start_count: usize,

    opts: &'a Options,
    counter: Box<dyn CounterTrait>,
}

impl<'a> Reporter<'a> {
    pub fn new(opts: &'a Options) -> Self {
        let log_file = if let Some(ref path) = opts.report_dir {
            let file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path.to_string() + "/timing.log")
                .unwrap();
            Some(file)
        } else {
            None
        };

        Reporter {
            start_time: Instant::now(),
            round_start_time: Instant::now(),
            round_start_count: 0,
            total_count: 0,
            log_file,
            opts,
            counter: Box::new(Counter::default()),
        }
    }

    pub fn set_counter<T: 'static + CounterTrait + Default>(&mut self) {
        self.counter = Box::new(T::default());
    }

    pub fn start(&mut self) {
        self.start_time = Instant::now();
        self.round_start_time = Instant::now();
        self.counter.reset();
    }

    pub fn notify_epoch(&mut self, epoch: usize, count: usize, db: &dyn AuthDB) {
        fn c(n: u64) -> String {
            let mut ans = String::new();
            ans.write_formatted(&n, &Locale::en).unwrap();
            ans
        }

        self.total_count += count;

        if (epoch + 1) % self.opts.report_epoch != 0 {
            return;
        }

        let count = self.total_count - self.round_start_count;

        let last = self.round_start_time.elapsed();
        let avg_time = last.as_secs_f64() / count as f64;

        let common = format!(
            "Time {:.3?} epoch {:?} > {} ops, {:.3?} us/op >",
            self.start_time.elapsed(),
            epoch + 1,
            c((1f64 / avg_time) as u64),
            avg_time * 1e6
        );
        let db_stat = {
            let stats = db.backend().io_stats(IoStatsKind::SincePrevious);
            let bytes_per_read = (stats.bytes_read as f64) / (stats.reads as f64);
            let bytes_per_write = (stats.bytes_written as f64) / (stats.writes as f64);
            let cached_rate = (stats.cache_reads as f64) / (stats.reads as f64);
            format!(
                "{} / {} r ({:.0}% cached) {} w, avg bytes {:.2}, {:.2} >",
                c(stats.reads),
                c(stats.cache_reads),
                cached_rate * 100.0,
                c(stats.writes),
                bytes_per_read,
                bytes_per_write,
            )
        };
        let customized = self.counter.report();
        println!("{} {} {}", common, db_stat, customized);

        if let Some(file) = &mut self.log_file {
            let _ = writeln!(
                file,
                "{},{},{:.3?}",
                self.opts.settings(),
                (epoch + 1) / self.opts.report_epoch,
                avg_time * 1e6
            );
        }
        self.round_start_time = Instant::now();
        self.round_start_count = self.total_count;
    }

    pub fn collect_profiling(&self, profiler: Profiler) {
        if self.opts.report_dir.is_none() {
            return;
        }

        let profile_prefix = self.opts.report_dir.as_ref().unwrap().clone()
            + "/"
            + &str::replace(&self.opts.settings(), ",", "_");
        profiler.report_to_file(&profile_prefix)
    }
}

pub trait CounterTrait {
    fn reset(&mut self) {}
    fn report(&mut self) -> String {
        "".to_string()
    }
}

#[derive(Default)]
pub struct Counter;

impl CounterTrait for Counter {}
