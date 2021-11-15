use super::Profiler;
use crate::Options;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

pub struct Reporter<'a> {
    pub start_time: Instant,
    total_count: usize,
    log_file: Option<File>,

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
        self.counter.reset();
    }

    pub fn notify_epoch(&mut self, epoch: usize, count: usize) {
        self.total_count += count;

        if (epoch + 1) % self.opts.report_epoch != 0 {
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

        if let Some(file) = &mut self.log_file {
            let _ = writeln!(
                file,
                "{},{},{:.3?}",
                self.opts.settings(),
                (epoch + 1) / self.opts.report_epoch,
                avg_time * 1e6
            );
        }
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
