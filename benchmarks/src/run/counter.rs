use super::Profiler;
use crate::{REPORT_DIR, REPORT_EPOCH};
use std::fs::File;
use std::io::Write;
use std::time::Instant;

pub struct Reporter {
    pub start_time: Instant,
    total_count: usize,
    file: File,
    settings: String,

    report_epoch: usize,
    counter: Box<dyn CounterTrait>,
}

impl Reporter {
    pub fn new(file: File, settings: String) -> Self {
        Reporter {
            start_time: Instant::now(),
            total_count: 0,
            file,
            settings,
            report_epoch: REPORT_EPOCH,
            counter: Box::new(Counter::default()),
        }
    }

    pub fn set_counter<T: 'static + CounterTrait + Default>(&mut self) {
        self.counter = Box::new(T::default());
    }

    pub fn start(&mut self) {
        self.start_time = Instant::now();
        self.counter.reset();
    }

    pub fn notify_epoch(&mut self, epoch: usize, count: usize) {
        self.total_count += count;

        if (epoch + 1) % self.report_epoch != 0 {
            return;
        }

        let last = self.start_time.elapsed();
        let avg_time = last.as_secs_f64() / self.total_count as f64;

        let common = format!(
            "Time {:.3?} epoch {:?} > {:.0} ops, {:.3?} us/op >",
            last,
            epoch + 1,
            1f64 / avg_time,
            avg_time * 1e6
        );
        let customized = self.counter.report();
        println!("{} {}", common, customized);

        let _ = writeln!(
            self.file,
            "{},{},{:.3?}",
            self.settings,
            (epoch + 1) / self.report_epoch,
            avg_time * 1e6
        );
    }

    pub fn collect_profiling(&self, profiler: Profiler) {
        let profile_prefix = REPORT_DIR.to_string() + "/" + &str::replace(&self.settings, ",", "_");
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
