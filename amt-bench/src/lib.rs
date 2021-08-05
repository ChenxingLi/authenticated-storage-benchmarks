#![feature(test)]
extern crate test;

mod basic_op;
mod db;

use rand::Rng;
use test::{black_box, Bencher};

#[bench]
fn random_gen(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();

    b.iter(|| {
        let (key, value): (u64, u64) = (rng.gen(), rng.gen());
        black_box((key, value));
    });
}
