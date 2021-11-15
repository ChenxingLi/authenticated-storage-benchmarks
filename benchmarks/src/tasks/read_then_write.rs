use super::*;
use crate::opts::Options;
use rand::prelude::*;

pub struct ReadThenWrite<R: Rng + SeedableRng> {
    pub total_keys: usize,
    pub batch_size: usize,
    pub random: R,
}

impl<R: Rng + SeedableRng> ReadThenWrite<R> {
    pub(crate) fn new(opts: &Options) -> Self {
        Self {
            total_keys: opts.total_keys,
            batch_size: opts.epoch_size,
            random: SeedableRng::seed_from_u64(opts.seed),
        }
    }
}

impl<R: Rng + SeedableRng> Iterator for ReadThenWrite<R> {
    type Item = Events;

    fn next(&mut self) -> Option<Self::Item> {
        let mut events = Vec::with_capacity(self.batch_size * 2);
        for _ in 0..self.batch_size {
            let integer = self.random.gen_range(0, self.total_keys);
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

impl<R: Rng + SeedableRng> TaskTrait for ReadThenWrite<R> {
    fn warmup(&mut self) {}
}
