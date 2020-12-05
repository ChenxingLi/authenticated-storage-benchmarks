use super::trusted_setup::PP;
use super::utils::{DEPTHS, LENGTH};
use algebra::bls12_381::{Bls12_381, Fr, G1Projective, G2Affine, G2Projective};
use algebra::{AffineCurve, FftField, Field, PairingEngine, Zero};
use algebra_core::fields::utils::k_adicity;
use ff_fft::{EvaluationDomain, Radix2EvaluationDomain};

use lazy_static::lazy_static;

#[cfg(test)]
use algebra::{One, ProjectiveCurve};

lazy_static! {
    pub static ref PUBLIC_PARAMETERS: Bls12_381_AMTPP =
        Bls12_381_AMTPP::from_file("./dat/pp.bin", DEPTHS);
}

pub trait AMTParams<PE: PairingEngine> {
    fn get_idents(&self, index: usize) -> &PE::G1Projective;
    fn get_quotient(&self, depth: usize, index: usize) -> &PE::G1Projective;
    fn get_g2_pow_tau(&self, index: usize) -> &PE::G2Projective;
    fn g2(&self) -> PE::G2Projective;
    fn w_inv(&self) -> PE::Fr;
}

pub struct Bls12_381_AMTPP {
    indents: Vec<G1Projective>,
    prove_cache: Vec<Vec<G1Projective>>,
    g2pp: Vec<G2Projective>,
    g2: G2Projective,
    w_inv: Fr,
}

impl AMTParams<Bls12_381> for Bls12_381_AMTPP {
    fn get_idents(&self, index: usize) -> &G1Projective {
        &self.indents[index]
    }

    fn get_quotient(&self, depth: usize, index: usize) -> &G1Projective {
        &self.prove_cache[depth - 1][index]
    }

    fn get_g2_pow_tau(&self, height: usize) -> &G2Projective {
        &self.g2pp[height]
    }

    fn g2(&self) -> G2Projective {
        self.g2.clone()
    }

    fn w_inv(&self) -> Fr {
        self.w_inv.clone()
    }
}

impl Bls12_381_AMTPP {
    fn from_file(pp_file: &str, depth: usize) -> Self {
        let (g1pp, g2pp) = PP::load_or_create_pp(pp_file).into_projective();

        assert_eq!(1 << g2pp.len(), g1pp.len());

        let fft_domain = Radix2EvaluationDomain::<Fr>::new(LENGTH).unwrap();

        let indents = fft_domain.fft(&g1pp[0..LENGTH]);

        let prove_cache: Vec<Vec<G1Projective>> = (1..=depth)
            .map(|d| gen_prove_cache(&g1pp[0..(1 << depth)], &fft_domain, d))
            .collect();

        let w_inv = Fr::get_root_of_unity(LENGTH).unwrap().inverse().unwrap();

        let g2 = G2Affine::prime_subgroup_generator().into_projective();

        Self {
            indents,
            prove_cache,
            g2pp,
            g2,
            w_inv,
        }
    }
}

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

#[test]
fn test_ident_prove() {
    const TEST_LEVEL: usize = DEPTHS;
    const TEST_LENGTH: usize = 1 << TEST_LEVEL;

    let (g1pp, g2pp) = PP::load_or_create_pp("dat/pp_test.bin").into_projective();

    let w: Fr = Fr::get_root_of_unity(TEST_LENGTH).unwrap();
    let w_inv: Fr = w.inverse().unwrap();
    assert_eq!(w.pow(&[TEST_LENGTH as u64]), Fr::one());

    let fft_domain = Radix2EvaluationDomain::<Fr>::new(TEST_LENGTH).unwrap();
    let indent_func = fft_domain.fft(&g1pp[0..TEST_LENGTH]);

    let g2 = G2Affine::prime_subgroup_generator().into_projective();

    for depth in 1..=TEST_LEVEL {
        let prove_data = gen_prove_cache(&g1pp[0..TEST_LENGTH], &fft_domain, depth);
        for i in 0..TEST_LENGTH {
            assert_eq!(
                Bls12_381::pairing(indent_func[i], g2),
                Bls12_381::pairing(
                    prove_data[i],
                    g2pp[TEST_LEVEL - depth]
                        + g2.mul(w_inv.pow([(i * (TEST_LENGTH >> depth)) as u64])),
                )
            );
        }
    }
}
