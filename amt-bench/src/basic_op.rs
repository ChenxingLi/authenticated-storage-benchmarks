use keccak_hash::keccak;
use rand::Rng;
use std::collections::{BTreeMap, HashMap};
use test::{black_box, Bencher};

#[bench]
fn add_u64_fold(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let (x, y): (u64, u64) = (rng.gen(), rng.gen());
    b.iter(|| {
        for _ in 0..1_000 {
            black_box(x + y);
        }
    });
}

#[bench]
fn hashmap_100_read_fold(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let mut map = HashMap::<u64, u64>::new();
    for i in 0..100 {
        map.insert(i * 736, rng.gen());
    }
    b.iter(|| {
        for i in 0..1_000 {
            let index = (i % 100 as u64) * 736;
            black_box(map.get(&index).unwrap());
        }
    });
}

#[bench]
fn hashmap_tuple_100_read_fold(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let mut map = HashMap::<(u64, u64, u64), u64>::new();
    for i in 0..1000 {
        map.insert((i * 736, i * 736 + 1, i * 736 + 2), rng.gen());
    }
    b.iter(|| {
        for i in 0..1_000 {
            let index = i * 736;
            black_box(map.get(&(index, index + 1, index + 2)).unwrap());
        }
    });
}

#[bench]
fn btreemap_10_read_fold(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let mut map = BTreeMap::<u64, u64>::new();
    for i in 0..10 {
        map.insert(i * 736, rng.gen());
    }
    b.iter(|| {
        for i in 0..1_000 {
            let index = (i % 10 as u64) * 736;
            black_box(map.get(&index).unwrap());
        }
    });
}

#[bench]
fn btreemap_100_read_fold(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let mut map = BTreeMap::<u64, u64>::new();
    for i in 0..100 {
        map.insert(i * 736, rng.gen());
    }
    b.iter(|| {
        for i in 0..1000 {
            let index = (i % 100 as u64) * 736;
            black_box(map.get(&index).unwrap());
        }
    });
}

#[bench]
fn btreemap_tuple_100_read_fold(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let mut map = BTreeMap::<(u64, u64, u64), u64>::new();
    for i in 0..1000 {
        map.insert((i * 736, i * 736 + 1, i * 736 + 2), rng.gen());
    }
    b.iter(|| {
        for i in 0..1_000 {
            let index = i * 736;
            black_box(map.get(&(index, index + 1, index + 2)).unwrap());
        }
    });
}

#[bench]
fn btreemap_1000_read_fold(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let mut map = BTreeMap::<u64, u64>::new();
    for i in 0..1000 {
        map.insert(i * 736, rng.gen());
    }
    b.iter(|| {
        for i in 0..1_000 {
            let index = i * 736;
            black_box(map.get(&index).unwrap());
        }
    });
}

#[bench]
fn mem_swap_read_fold(b: &mut Bencher) {
    type BigInt = FrInt<Pairing>;
    let mut map1 = (vec![0u64], vec![3u64], BigInt::from(3));
    let mut map2 = (vec![1u64], vec![0u64], BigInt::from(9));
    b.iter(|| {
        for _ in 0..1_000 {
            ::std::mem::swap(&mut map1, &mut map2);
            black_box(map1.0[0]);
        }
    });
}

use amt_db::crypto::export::{BigInteger, FrInt, G1Aff, Pairing, ProjectiveCurve, UniformRand, G1};

#[bench]
fn muln_fold(b: &mut Bencher) {
    type BigInt = FrInt<Pairing>;
    let mut x = BigInt::from(1);
    b.iter(|| {
        for _ in 0..1_000 {
            black_box(x.muln(5));
        }
    });
}

#[bench]
fn projective_transform(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let x = G1::<Pairing>::rand(&mut rng);
    let y = G1::<Pairing>::rand(&mut rng);
    let z = x + y;
    b.iter(|| {
        let az: G1Aff<Pairing> = z.clone().into();
        black_box(az);
    });
}

#[bench]
fn projective_transform_batch(b: &mut Bencher) {
    let mut rng = ::rand::thread_rng();
    let mut tasks = Vec::<G1<Pairing>>::with_capacity(1000);
    for i in 0..100 {
        let x = G1::<Pairing>::rand(&mut rng);
        let y = G1::<Pairing>::rand(&mut rng);
        let z = x + y;
        tasks.push(z);
    }
    b.iter(|| {
        let mut answer = tasks.clone();
        ProjectiveCurve::batch_normalization(&mut answer);
        black_box(answer);
    });
}

#[bench]
fn keccak_fold(b: &mut Bencher) {
    b.iter(|| {
        for i in 0u16..1_000 {
            let value = [i; 16];
            let value = unsafe { std::mem::transmute::<[u16; 16], [u8; 32]>(value) };
            let ans = keccak(&value);
            black_box(ans);
        }
    });
}
