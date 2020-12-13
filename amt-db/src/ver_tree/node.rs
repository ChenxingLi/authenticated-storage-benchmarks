use super::Key;
use crate::amt::{
    paring_provider::{Fr as FrGeneric, FrInt as FrIntGeneric, Pairing},
    AMTData,
};
use algebra::{
    CanonicalDeserialize, CanonicalSerialize, FpParameters, PrimeField, Read, SerializationError,
    Write,
};

pub(super) type Fr = FrGeneric<Pairing>;
pub(super) type FrInt = FrIntGeneric<Pairing>;

#[allow(dead_code)]
fn const_assert() {
    const CAPACITY: u32 = <Fr as PrimeField>::Params::CAPACITY;
    const_assert!(CAPACITY > 40 * 6);
}

#[derive(Default, Clone, CanonicalDeserialize, CanonicalSerialize)]
pub struct Node {
    pub(super) key_versions: Vec<(Key, u64)>,
    pub(super) tree_version: u64,
}

impl AMTData<Fr> for Node {
    #[cfg(target_endian = "little")]
    fn as_fr_int(&self) -> FrInt {
        assert!(self.key_versions.len() <= 5);
        let mut result = [0u8; 32];

        let mut start = 5usize;
        for (_, ver) in self.key_versions.iter() {
            result[start..(start + 5)].copy_from_slice(&ver.to_le_bytes()[0..5]);
            start += 5;
        }
        result[0..5].copy_from_slice(&self.tree_version.to_le_bytes()[0..5]);

        let result = unsafe { std::mem::transmute::<[u8; 32], [u64; 4]>(result) };
        FrInt::new(result)
    }
}

#[test]
#[cfg(target_endian = "little")]
fn test_array_convert() {
    use std::convert::TryInto;

    let origin: [u8; 32] = (0..32).collect::<Vec<u8>>().try_into().unwrap();
    let ans_f = |x: usize| (0..8).map(|y| (x + y) * (1 << (8 * y))).sum::<usize>() as u64;
    let answer: [u64; 4] = [ans_f(0), ans_f(8), ans_f(16), ans_f(24)];
    let converted = unsafe { std::mem::transmute::<[u8; 32], [u64; 4]>(origin) };
    assert_eq!(converted, answer);
}

#[test]
fn test_array_transmute() {
    let mut node = Node {
        key_versions: Vec::new(),
        tree_version: 0,
    };
    node.tree_version = 1;
    (2..=6).for_each(|x: u64| node.key_versions.push((Key::default(), x)));

    let mut answer = [0u64; 4];
    answer[0] = 1;
    answer[0] += 2 * (1 << 40);
    answer[1] += 3 * (1 << 80 - 64);
    answer[1] += 4 * (1 << 120 - 64);
    answer[2] += 5 * (1 << 160 - 128);
    answer[3] += 6 * (1 << 200 - 192);
    let answer = FrInt::new(answer);

    assert_eq!(node.as_fr_int(), answer);
}

#[cfg(test)]
use rand::{prelude::ThreadRng, Rng};
#[cfg(test)]
fn test_random_node_as_fr_int(rng: &mut ThreadRng) {
    use algebra::BigInteger;

    let mut node = Node {
        key_versions: vec![Default::default(); 5],
        tree_version: 0,
    };

    const MASK: u64 = (1 << 40) - 1;

    node.tree_version = rng.gen::<u64>() & MASK;
    let mut answer = FrInt::from(node.tree_version);
    for i in 0..5 {
        node.key_versions[i].1 = rng.gen::<u64>() & MASK;
        let mut fr_int = FrInt::from(node.key_versions[i].1);
        fr_int.muln(40 * (i as u32 + 1));
        answer.add_nocarry(&fr_int);
    }

    assert_eq!(node.as_fr_int(), answer);
}

#[test]
fn test_as_fr_int() {
    let mut rng = rand::thread_rng();
    for _ in 0..1000 {
        test_random_node_as_fr_int(&mut rng);
    }
}
