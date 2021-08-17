#![allow(unused_imports, dead_code, unused_variables)]
use crate::SEED;
use cfx_types::{H256, U256};
use crc64fast::Digest as Hasher;
use keccak_hash::keccak;
use pprof::{self, protos::Message, ProfilerGuard};
use rand::{prelude::*, RngCore};
use rand_pcg::Pcg64;
use std::fs::File;
use std::io::Write;
use std::ops::Sub;
use std::time::{Duration, Instant};

type Key = Vec<u8>;
type Value = Vec<u8>;

pub enum Event {
    Read(Key),
    Write(Key, Value),
}

pub struct Events(pub Vec<Event>);

pub struct ReadThenWrite<R: Rng + SeedableRng> {
    pub total_keys: usize,
    pub batch_size: usize,
    pub random: R,
}

impl<R: Rng + SeedableRng> ReadThenWrite<R> {
    pub(crate) fn new(total_keys: usize, batch_size: usize) -> Self {
        Self {
            total_keys,
            batch_size,
            random: SeedableRng::seed_from_u64(SEED),
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

fn hash(input: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.write(input);
    let checksum = hasher.sum64();
    unsafe { std::mem::transmute::<[u64; 4], [u8; 32]>([checksum; 4]) }
}
