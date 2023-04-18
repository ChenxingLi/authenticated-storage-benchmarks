use super::*;
use asb_options::Options;
use rand::prelude::*;
use std::marker::PhantomData;

pub struct ReadThenWrite<R: Rng + SeedableRng> {
    pub total_keys: usize,
    pub batch_size: usize,
    pub seed: u64,
    _phantom: PhantomData<R>,
}

impl<R: Rng + SeedableRng> ReadThenWrite<R> {
    pub fn new(opts: &Options) -> Self {
        Self {
            total_keys: opts.total_keys,
            batch_size: opts.epoch_size,
            seed: opts.seed,
            _phantom: PhantomData,
        }
    }
}

pub struct ReadThenWriteTaskIter<'a, R: Rng + SeedableRng> {
    inner: &'a ReadThenWrite<R>,
    random: R,
}

impl<R: Rng + SeedableRng> Iterator for ReadThenWriteTaskIter<'_, R> {
    type Item = Events;

    fn next(&mut self) -> Option<Self::Item> {
        let mut events = Vec::with_capacity(self.inner.batch_size * 2);
        for _ in 0..self.inner.batch_size {
            let integer = self.random.gen_range(0, self.inner.total_keys);
            let key = hash(&integer.to_be_bytes()).to_vec();
            events.push(Event::Read(key.clone()));
            events.push(Event::Write(
                key.clone(),
                self.random.gen::<[u8; 32]>().to_vec(),
            ));
        }

        Some(Events(events))
    }
}

pub struct ReadThenWriteWarmupIter<'a, R: Rng + SeedableRng> {
    inner: &'a ReadThenWrite<R>,
    random: R,
    keys: Vec<usize>,
}

impl<R: Rng + SeedableRng> Iterator for ReadThenWriteWarmupIter<'_, R> {
    type Item = Events;

    fn next(&mut self) -> Option<Self::Item> {
        let mut task_keys = Vec::with_capacity(self.inner.batch_size);
        for _ in 0..self.inner.batch_size {
            if let Some(v) = self.keys.pop() {
                task_keys.push(v);
            } else {
                break;
            }
        }
        if task_keys.is_empty() {
            return None;
        }
        let mut events = Vec::with_capacity(task_keys.len());
        for key in task_keys.into_iter() {
            let key = hash(&key.to_be_bytes()).to_vec();
            events.push(Event::Write(
                key.clone(),
                self.random.gen::<[u8; 32]>().to_vec(),
            ));
        }
        Some(Events(events))
    }
}

impl<R: Rng + SeedableRng> TaskTrait for ReadThenWrite<R> {
    fn warmup<'a>(&'a self) -> Box<dyn Iterator<Item = Events> + 'a> {
        let mut random = R::seed_from_u64(self.seed + 1);
        let mut keys: Vec<usize> = (0..self.total_keys).collect();
        keys.shuffle(&mut random);
        Box::new(ReadThenWriteWarmupIter {
            inner: &self,
            random,
            keys,
        })
    }

    fn tasks<'a>(&'a self) -> Box<dyn Iterator<Item = Events> + 'a> {
        Box::new(ReadThenWriteTaskIter {
            inner: &self,
            random: R::seed_from_u64(self.seed),
        })
    }
}
