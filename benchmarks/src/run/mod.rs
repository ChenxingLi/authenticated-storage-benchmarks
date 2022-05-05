pub mod counter;
pub mod profiler;

use crate::db::AuthDB;
use crate::opts::Options;
use crate::tasks::{Event, Events, TaskTrait};
use fs_extra::dir::CopyOptions;
use kvdb::IoStatsKind;
use std::any::Any;
use std::fs;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

pub use counter::{CounterTrait, Reporter};
pub use profiler::Profiler;

fn warmup(db: &mut dyn AuthDB, tasks: Box<dyn Iterator<Item = Events> + '_>, opts: &Options) {
    let time = Instant::now();

    for (epoch, events) in tasks.enumerate() {
        for event in events.0.into_iter() {
            if let Event::Write(key, value) = event {
                db.set(key, value);
            }
        }
        db.commit(epoch);
        if (epoch + 1) % opts.report_epoch == 0 {
            println!(
                "Time {:>7.3?}s, Warming up epoch: {:>5}",
                time.elapsed().as_secs_f64(),
                epoch + 1
            );
        }
    }

    db.flush_all();
    db.backend().io_stats(IoStatsKind::SincePrevious);
}

pub fn run_tasks(
    mut db: Box<dyn AuthDB>,
    _backend_any: Arc<dyn Any>,
    tasks: impl TaskTrait,
    mut reporter: Reporter,
    opts: &Options,
) {
    println!("Start warming up");
    if opts.warmup_from.is_none() && !opts.no_warmup {
        warmup(&mut *db, tasks.warmup(), opts);
        if let Some(ref warmup_dir) = opts.warmup_to() {
            println!("Waiting for post ops");

            sleep(Duration::from_secs_f64(f64::max(
                1.0,
                opts.total_keys as f64 / 1e6,
            )));

            let _ = fs::remove_dir_all(warmup_dir);
            fs::create_dir_all(warmup_dir).unwrap();

            let mut copy_options = CopyOptions::new();
            copy_options.overwrite = true;
            copy_options.copy_inside = true;
            copy_options.content_only = true;
            println!("Writing warmup to {}", warmup_dir);
            fs_extra::dir::copy(&opts.db_dir, warmup_dir, &copy_options).unwrap();
            println!("Writing done");
            return;
        }
    }
    println!("Warm up done");

    let frequency = if opts.report_dir.is_none() { -1 } else { 250 };
    let mut profiler = Profiler::new(frequency);
    reporter.start();

    for (epoch, events) in tasks.tasks().enumerate() {
        if reporter.start_time.elapsed().as_secs() >= opts.max_time.unwrap_or(u64::MAX)
            || epoch + 1 >= opts.max_epoch.unwrap_or(usize::MAX)
        {
            profiler.tick();
            break;
        }

        if (epoch + 1) % opts.profile_epoch == 0 {
            profiler.tick();
        }

        let count = events.0.len();

        for event in events.0.into_iter() {
            match event {
                Event::Read(key) => {
                    let ans = db.get(key);
                    if ans.is_none() {
                        reporter.notify_empty_read();
                    }
                }
                Event::Write(key, value) => {
                    db.set(key, value);
                }
            }
        }
        db.commit(epoch);

        reporter.notify_epoch(epoch, count, &*db, opts);
    }

    reporter.collect_profiling(profiler);
}
