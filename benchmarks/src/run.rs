use crate::task_producer::{Event, Events};
use std::time::Instant;

pub fn run_tasks(
    db: &mut dyn BenchmarkDB,
    epochs: impl Iterator<Item = Events>,
    recorder: &mut dyn CounterTrait,
    total_seconds: u64,
) {
    let time = Instant::now();
    let mut total_count = 0;
    recorder.reset();
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
        if (epoch + 1) % 50 == 0 {
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
        }

        if time.elapsed().as_secs() >= total_seconds {
            break;
        }
    }
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
