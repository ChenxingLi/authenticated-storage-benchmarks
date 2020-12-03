use super::utils::{DEPTHS, LENGTH};
use algebra::bls12_381::{Bls12_381, Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use algebra::{
    AffineCurve, CanonicalDeserialize, CanonicalSerialize, FftField, Field, FpParameters,
    FromBytes, One, PairingEngine, ProjectiveCurve, UniformRand, Zero,
};
use algebra_core::fields::utils::k_adicity;
use ff_fft::{EvaluationDomain, Radix2EvaluationDomain};
use rand;
use std::fs::File;
use std::ops::MulAssign;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref PUBLIC_PARAMETERS: Bls12_381_AMTPP = Bls12_381_AMTPP::load_test_data();
}

pub trait AMTParams<PE: PairingEngine> {
    fn get_idents(&self, index: usize) -> &PE::G1Projective;
    fn get_prove_cache(&self, depth: usize, index: usize) -> &PE::G1Projective;
    fn get_verification(&self, index: usize) -> PE::G2Projective;

    fn g2(&self) -> PE::G2Projective;
    fn w_inv(&self) -> PE::Fr;
}

pub struct Bls12_381_AMTPP {
    indents: Vec<G1Projective>,
    prove_cache: Vec<Vec<G1Projective>>,
    g2pp: Vec<G2Projective>,
    w_inv: Fr,
}

impl AMTParams<Bls12_381> for Bls12_381_AMTPP {
    fn get_idents(&self, index: usize) -> &G1Projective {
        &self.indents[index]
    }

    fn get_prove_cache(&self, depth: usize, index: usize) -> &G1Projective {
        &self.prove_cache[depth - 1][index]
    }

    fn get_verification(&self, height: usize) -> G2Projective {
        self.g2pp[height + 1].clone()
    }

    fn g2(&self) -> G2Projective {
        self.g2pp[0].clone()
    }

    fn w_inv(&self) -> Fr {
        self.w_inv.clone()
    }
}

impl Bls12_381_AMTPP {
    fn load_test_data() -> Self {
        Self {
            indents: load_and_gen_idents(),
            prove_cache: load_and_gen_all_prove_cache(),
            g2pp: load_g2pp(),
            w_inv: Fr::get_root_of_unity(LENGTH).unwrap().inverse().unwrap(),
        }
    }
}

pub fn trusted_setup(depth: usize) -> (Fr, Vec<G1Affine>, Vec<G2Affine>) {
    let tau: Fr = Fr::rand(&mut rand::thread_rng());
    let mut gen = G1Affine::prime_subgroup_generator().into_projective();
    let gen2 = G2Affine::prime_subgroup_generator().into_projective();

    let mut g1pp = vec![];
    g1pp.reserve(1 << depth);
    for _ in 0..1 << depth {
        g1pp.push(gen.into_affine());
        gen.mul_assign(tau.clone());
    }

    let mut g2pp: Vec<G2Affine> = vec![];
    let mut e = tau.clone();
    g2pp.reserve(depth + 1);
    g2pp.push(gen2.into_affine());
    for _ in 0..depth {
        let value: G2Projective = gen2.mul(e.clone());
        g2pp.push(value.into_affine());
        e.square_in_place();
    }

    return (tau, g1pp, g2pp);
}

#[allow(unused)]
pub fn dump_pp(file: &'static str, depth: usize) -> () {
    let buffer = File::create(file).unwrap();
    trusted_setup(depth).serialize_uncompressed(&buffer);
}

pub fn load_pp(file: &'static str) -> PP {
    let buffer = File::open(file).unwrap();
    let (tau, g1pp_affine, g2pp_affine) =
        <(Fr, Vec<G1Affine>, Vec<G2Affine>) as CanonicalDeserialize>::deserialize_unchecked(buffer)
            .unwrap();
    let g1pp: Vec<G1Projective> = g1pp_affine
        .iter()
        .copied()
        .map(|x| G1Projective::from(x))
        .collect();

    let g2pp: Vec<G2Projective> = g2pp_affine
        .iter()
        .copied()
        .map(|x| G2Projective::from(x))
        .collect();
    return (Some(tau), g1pp, g2pp);
}

type PP = (Option<Fr>, Vec<G1Projective>, Vec<G2Projective>); // In the debug case, we include Fr

pub fn gen_prove_cache(
    g1pp: &[G1Projective],
    fft_domain: &Radix2EvaluationDomain<Fr>,
    depth: usize,
) -> Vec<G1Projective> {
    assert!(g1pp.len() <= 1 << 32);

    let length = g1pp.len();
    let max_depth = k_adicity(2, length) as usize;

    assert_eq!(1 << max_depth, length);
    assert!(max_depth >= depth);
    assert!(depth >= 1);

    let chunk_length = (1 << (max_depth - depth)) as usize;
    let chunk_num = length / chunk_length;

    let mut g1pp_chunks_iter = g1pp.chunks(1 << (max_depth - depth) as usize);
    let mut coeff = vec![G1Projective::zero(); length];

    for i in 0..(chunk_num / 2) {
        coeff[(2 * i + 1) * chunk_length..(2 * i + 2) * chunk_length]
            .copy_from_slice(g1pp_chunks_iter.next().unwrap());
        g1pp_chunks_iter.next();
    }

    return fft_domain.fft(&coeff);
}

pub fn load_and_gen_all_prove_cache() -> Vec<Vec<G1Projective>> {
    let (_, g1pp, _) = load_pp("dat/pp_bls12_381_small.bin");

    let fft_domain = Radix2EvaluationDomain::<Fr>::new(LENGTH).unwrap();

    let prove_datas: Vec<Vec<G1Projective>> = (1..=DEPTHS)
        .map(|depth| gen_prove_cache(&g1pp[0..LENGTH], &fft_domain, depth))
        .collect();

    return prove_datas;
}

pub fn load_and_gen_idents() -> Vec<G1Projective> {
    let (_, g1pp, _) = load_pp("dat/pp_bls12_381_small.bin");

    let fft_domain = Radix2EvaluationDomain::<Fr>::new(LENGTH).unwrap();

    fft_domain.fft(&g1pp[0..LENGTH])
}

pub fn load_g2pp() -> Vec<G2Projective> {
    let (_, _, g2pp) = load_pp("dat/pp_bls12_381_small.bin");
    g2pp
}

#[test]
fn test_pairing() {
    let (_, g1pp, g2pp) = load_pp("dat/pp_bls12_381_small.bin");
    assert_eq!(
        Bls12_381::pairing(g1pp[8], g2pp[0]),
        Bls12_381::pairing(g1pp[0], g2pp[4])
    );
}

#[test]
fn test_ident_prove() {
    const TEST_LEVEL: usize = 6;
    const TEST_LENGTH: usize = 1 << TEST_LEVEL;

    let (_, g1pp, g2pp) = load_pp("dat/pp_bls12_381_small.bin");

    let w: Fr = Fr::get_root_of_unity(TEST_LENGTH).unwrap();
    let w_inv: Fr = w.inverse().unwrap();
    assert_eq!(w.pow(&[TEST_LENGTH as u64]), Fr::one());

    let fft_domain = Radix2EvaluationDomain::<Fr>::new(TEST_LENGTH).unwrap();
    let indet_func = fft_domain.fft(&g1pp[0..TEST_LENGTH]);

    let g2 = g2pp[0].clone();

    for depth in 1..=TEST_LEVEL {
        let prove_data = gen_prove_cache(&g1pp[0..TEST_LENGTH], &fft_domain, depth);
        for i in 0..TEST_LENGTH {
            assert_eq!(
                Bls12_381::pairing(indet_func[i], g2),
                Bls12_381::pairing(
                    prove_data[i],
                    g2pp[1 + TEST_LEVEL - depth]
                        + g2.mul(w_inv.pow([(i * (TEST_LENGTH >> depth)) as u64])),
                )
            );
        }
    }
}
