#![feature(test)]
extern crate test;

mod basic_op;

use amt_db::storage::{open_col, KeyValueDbTrait, KeyValueDbTraitRead};
use rand::Rng;
use test::{black_box, Bencher};

#[bench]
fn random_gen(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let db = open_col("./__benchmark", 0u32);

    b.iter(|| {
        let (key, value): (u64, u64) = (rng.gen(), rng.gen());
        black_box((key, value));
    });

    ::std::fs::remove_dir_all("./__benchmark").unwrap();
}

#[bench]
fn db_write(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let db = open_col("./__benchmark", 0u32);

    b.iter(|| {
        let (key, value): (u64, u64) = (rng.gen(), rng.gen());
        db.put(&key.to_be_bytes(), &value.to_be_bytes()).unwrap();
        let load = db.get(&key.to_be_bytes()).unwrap();
        black_box(load)
    });

    ::std::fs::remove_dir_all("./__benchmark").unwrap();
}

#[bench]
fn db_write_large(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let db = open_col("./__benchmark", 0u32);

    b.iter(|| {
        let (key, value): (u64, [u64; 32]) = (rng.gen(), rng.gen());
        db.put(&key.to_be_bytes(), &unsafe {
            std::mem::transmute::<[u64; 32], [u8; 256]>(value)
        })
        .unwrap();
        let load = db.get(&key.to_be_bytes()).unwrap();
        black_box(load)
    });

    ::std::fs::remove_dir_all("./__benchmark").unwrap();
}

#[bench]
fn db_read(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let db = open_col("./__benchmark", 0u32);

    for i in 0..100000 {
        let (key, value): (u32, u64) = (i, rng.gen());
        db.put(&key.to_be_bytes(), &value.to_be_bytes()).unwrap();
    }

    b.iter(|| {
        let key: u32 = rng.gen::<u32>() % 100000;
        let load = db.get(&key.to_be_bytes()).unwrap();
        black_box(load)
    });

    ::std::fs::remove_dir_all("./__benchmark").unwrap();
}

#[bench]
fn db_read_1m(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let db = open_col("./__benchmark", 0u32);

    for i in 0..1000000 {
        let (key, value): (u32, u64) = (i, rng.gen());
        db.put(&key.to_be_bytes(), &value.to_be_bytes()).unwrap();
    }

    b.iter(|| {
        let key: u32 = rng.gen::<u32>() % 1000000;
        let load = db.get(&key.to_be_bytes()).unwrap();
        black_box(load)
    });

    ::std::fs::remove_dir_all("./__benchmark").unwrap();
}

#[bench]
fn db_read_10m(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let db = open_col("./__benchmark", 0u32);

    for i in 0..10000000 {
        let (key, value): (u32, u64) = (i, rng.gen());
        db.put(&key.to_be_bytes(), &value.to_be_bytes()).unwrap();
    }

    b.iter(|| {
        let key: u32 = rng.gen::<u32>() % 10000000;
        let load = db.get(&key.to_be_bytes()).unwrap();
        black_box(load)
    });

    ::std::fs::remove_dir_all("./__benchmark").unwrap();
}

#[bench]
fn db_read_30m(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let db = open_col("./__benchmark", 0u32);

    let mut data = rng.gen();

    for i in 0..30000000 {
        let (key, value): (u32, u64) = (i, data);
        db.put(&key.to_be_bytes(), &value.to_be_bytes()).unwrap();
    }

    b.iter(|| {
        let key: u32 = rng.gen::<u32>() % 30000000;
        let load = db.get(&key.to_be_bytes()).unwrap();
        black_box(load)
    });

    ::std::fs::remove_dir_all("./__benchmark").unwrap();
}

#[bench]
fn db_read_large(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let db = open_col("./__benchmark", 0u32);

    for i in 0..100000 {
        let (key, value): (u32, [u64; 32]) = (i, rng.gen());
        db.put(&key.to_be_bytes(), &unsafe {
            std::mem::transmute::<[u64; 32], [u8; 256]>(value)
        })
        .unwrap();
    }

    b.iter(|| {
        let key: u32 = rng.gen::<u32>() % 100000;
        let load = db.get(&key.to_be_bytes()).unwrap();
        black_box(load)
    });

    ::std::fs::remove_dir_all("./__benchmark").unwrap();
}

#[bench]
fn db_read_30m_large(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let db = open_col("./__benchmark", 0u32);

    for i in 0..30000000 {
        let (key, value): (u32, [u64; 32]) = (i, rng.gen());
        db.put(&key.to_be_bytes(), &unsafe {
            std::mem::transmute::<[u64; 32], [u8; 256]>(value)
        })
        .unwrap();
    }

    b.iter(|| {
        let key: u32 = rng.gen::<u32>() % 30000000;
        let load = db.get(&key.to_be_bytes()).unwrap();
        black_box(load)
    });

    ::std::fs::remove_dir_all("./__benchmark").unwrap();
}
