use crate::task_producer::{Event, Events};
use crate::{MAX_EPOCH, PROFILE_EPOCH, REPORT_DIR, REPORT_EPOCH, SECONDS};
use pprof::protos::Message;
use pprof::{ProfilerGuard, Report};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

pub struct WrappedProfilerGuard {
    inner: Option<ProfilerGuard<'static>>,
    frequency: i32,
    reports: Vec<Report>,
}

impl WrappedProfilerGuard {
    fn new(frequency: i32) -> Self {
        Self {
            inner: Some(pprof::ProfilerGuard::new(frequency).unwrap()),
            frequency,
            reports: Vec::new(),
        }
    }

    fn tick(&mut self) {
        let profiler = std::mem::take(&mut self.inner).unwrap();
        let report = profiler.report().build().unwrap();
        self.reports.push(report);
        std::mem::drop(profiler);
        self.inner = Some(pprof::ProfilerGuard::new(self.frequency).unwrap())
    }

    fn report(self, prefix: &str) {
        print!("Writing profiles... ");

        for (index, report) in self.reports.into_iter().enumerate() {
            let path = format!("{}_{:02}.pb", prefix, index);
            let mut file = File::create(path).unwrap();
            let profile = report.pprof().unwrap();

            let mut content = Vec::new();
            profile.encode(&mut content).unwrap();
            file.write_all(&content).unwrap();
        }
        println!("Done");
    }
}

pub fn run_tasks(
    db: &mut dyn BenchmarkDB,
    epochs: impl Iterator<Item = Events>,
    recorder: &mut dyn CounterTrait,
    mut file: File,
    setting_prefix: String,
) {
    let time = Instant::now();
    let mut total_count = 0;
    recorder.reset();
    let mut profiler = WrappedProfilerGuard::new(250);

    for (epoch, events) in epochs.enumerate() {
        let length = events.0.len();
        for event in events.0.into_iter() {
            match event {
                Event::Read(key) => {
                    db.get(key);
                }
                Event::Write(key, value) => {
                    db.set(key, value);
                }
            }
        }
        db.commit(epoch);
        total_count += length;

        // Report task
        if (epoch + 1) % REPORT_EPOCH == 0 {
            let last = time.elapsed();
            let avg_time = last.as_secs_f64() / total_count as f64;
            let prefix = format!(
                "Time {:.3?} epoch {:?} > {:.0} ops, {:.3?} us/op >",
                time.elapsed(),
                epoch + 1,
                1f64 / avg_time,
                avg_time * 1e6
            );
            let customized = recorder.mark();
            println!("{} {}", prefix, customized);
            writeln!(
                &mut file,
                "{},{},{:.3?}",
                setting_prefix,
                (epoch + 1) / REPORT_EPOCH,
                avg_time * 1e6
            );
        }

        if (epoch + 1) % PROFILE_EPOCH == 0 {
            profiler.tick();
        }

        if time.elapsed().as_secs() >= SECONDS || epoch + 1 >= MAX_EPOCH {
            profiler.tick();
            break;
        }
    }

    let profile_prefix = REPORT_DIR.to_string() + "/" + &str::replace(&setting_prefix, ",", "_");
    profiler.report(&profile_prefix)
}

pub trait BenchmarkDB {
    fn get(&self, key: Vec<u8>) -> Option<Box<[u8]>>;
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>);
    fn commit(&mut self, index: usize);
}

pub trait CounterTrait {
    fn reset(&mut self);
    fn mark(&mut self) -> String;
}

pub struct Counter;

impl CounterTrait for Counter {
    fn reset(&mut self) {}

    fn mark(&mut self) -> String {
        "".to_string()
    }
}

impl Counter {
    pub fn new() -> Self {
        Self
    }
}
