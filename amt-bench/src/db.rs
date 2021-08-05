use amt_db::storage::{open_col, KeyValueDbTrait, KeyValueDbTraitRead};
use rand::Rng;
use test::{black_box, Bencher};

#[bench]
fn write(b: &mut Bencher) {
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
fn write_large(b: &mut Bencher) {
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
fn read(b: &mut Bencher) {
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
fn read_1m(b: &mut Bencher) {
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
fn read_10m(b: &mut Bencher) {
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
fn read_10m_fold(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let db = open_col("./__benchmark", 0u32);

    for i in 0..10000000 {
        let (key, value): (u32, u64) = (i, rng.gen());
        db.put(&key.to_be_bytes(), &value.to_be_bytes()).unwrap();
    }

    b.iter(|| {
        for _ in 0..100000 {
            let key: u32 = rng.gen::<u32>() % 10000000;
            let load = db.get(&key.to_be_bytes()).unwrap();
            black_box(load);
        }
    });

    ::std::fs::remove_dir_all("./__benchmark").unwrap();
}

#[bench]
fn read_10m_seq(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let db = open_col("./__benchmark", 0u32);

    for i in 0..10000000 {
        let (key, value): (u32, u64) = (i, rng.gen());
        db.put(&key.to_be_bytes(), &value.to_be_bytes()).unwrap();
    }

    b.iter(|| {
        let mut x = rng.gen::<u32>() % 10000000;
        for _ in 0..100000 {
            let load = db.get(&x.to_be_bytes()).unwrap();
            let mut answer = [0u8; 8];
            answer.copy_from_slice(load.unwrap().as_ref());
            x = (u64::from_be_bytes(answer) % 10000000) as u32;
        }
        black_box(x)
    });

    ::std::fs::remove_dir_all("./__benchmark").unwrap();
}

#[bench]
fn read_30m(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let db = open_col("./__benchmark", 0u32);

    for i in 0..30000000 {
        let (key, value): (u32, u64) = (i, rng.gen());
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
fn read_large(b: &mut Bencher) {
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
