pub mod counter;
pub mod profiler;

use crate::db::AuthDB;
use crate::opts::Options;
use crate::tasks::{Event, TaskTrait};
use std::time::Instant;

pub use counter::{CounterTrait, Reporter};
pub use profiler::Profiler;

pub fn run_tasks(
    mut db: Box<dyn AuthDB>,
    mut tasks: impl TaskTrait,
    mut reporter: Reporter,
    opts: &Options,
) {
    println!("Start warming up");
    let time = Instant::now();
    tasks.warmup();
    println!(
        "Warming up takes {:.3?} seconds",
        time.elapsed().as_secs_f64()
    );

    let mut profiler = Profiler::new(250);
    reporter.start();

    for (epoch, events) in tasks.enumerate() {
        let count = events.0.len();

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

        reporter.notify_epoch(epoch, count);

        if reporter.start_time.elapsed().as_secs() >= opts.max_time
            || epoch + 1 >= opts.max_epoch.unwrap_or(usize::MAX)
        {
            profiler.tick();
            break;
        }

        if (epoch + 1) % opts.profile_epoch == 0 {
            profiler.tick();
        }
    }

    reporter.collect_profiling(profiler);
}
