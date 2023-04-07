#![allow(unused)]
use std::path::Path;

use postcard::from_bytes;
use serde::{Deserialize, Serialize};

use crate::opts::Options;

use super::{Event, Events, TaskTrait};

#[derive(Clone, Debug, Serialize, Deserialize)]
enum ExperimentTask {
    Read([u8; 32]),
    Write([u8; 32], Bytes),
}

type Bytes = Vec<u8>;
type InitTasks = Vec<([u8; 32], Bytes)>;
type IoTasks = Vec<Vec<ExperimentTask>>;

fn read_from_file<T, S: AsRef<Path>>(path: S) -> T
where
    for<'a> T: Deserialize<'a>,
{
    let loaded = std::fs::read(path.as_ref()).unwrap();
    from_bytes(&loaded).unwrap()
}

pub struct RealTrace {
    init_tasks: InitTasks,
    io_tasks: IoTasks,
}

impl RealTrace {
    pub fn load(opt: &Options) -> Self {
        let path = Path::new(&opt.trace_dir);
        let init_tasks = read_from_file(path.join("real_trace.init").as_os_str());
        let io_tasks = read_from_file(path.join("real_trace.data").as_os_str());
        RealTrace {
            init_tasks,
            io_tasks,
        }
    }
}

impl TaskTrait for RealTrace {
    fn tasks<'a>(&'a self) -> Box<dyn Iterator<Item = Events> + 'a> {
        Box::new(self.io_tasks.iter().map(|block_io| {
            Events(
                block_io
                    .iter()
                    .map(|io| match io {
                        ExperimentTask::Read(key) => Event::Read(key.to_vec()),
                        ExperimentTask::Write(key, value) => {
                            Event::Write(key.to_vec(), value.clone())
                        }
                    })
                    .collect::<Vec<_>>(),
            )
        }))
    }

    fn warmup<'a>(&'a self) -> Box<dyn Iterator<Item = Events> + 'a> {
        Box::new(self.init_tasks.chunks(1000).map(|arr| {
            Events(
                arr.iter()
                    .map(|(key, value)| Event::Write(key.to_vec(), value.clone()))
                    .collect::<Vec<_>>(),
            )
        }))
    }
}
