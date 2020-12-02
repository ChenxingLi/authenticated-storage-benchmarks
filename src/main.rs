#![allow(dead_code, unused_imports, non_camel_case_types)]

mod amt;
mod complete_tree;
pub mod public_parameters;
pub mod utils;

use cfx_storage;

// #[macro_use]
// extern crate lazy_static;
use algebra::bls12_381::{Fr, G1Projective, G2Projective};
use bencher::black_box;
use ff_fft::{EvaluationDomain, Radix2EvaluationDomain};

use public_parameters::{gen_prove_cache, load_pp};
use utils::{LENGTH, LEVELS};

fn main() {
    let (_, g1pp, _) = load_pp("dat/pp_bls12_381_small.bin");

    let fft_domain = Radix2EvaluationDomain::<Fr>::new(LENGTH).unwrap();

    let indet_func = fft_domain.fft(&g1pp[0..LENGTH]);

    let prove_datas: Vec<Vec<G1Projective>> = (1..=LEVELS)
        .map(|level| gen_prove_cache(&g1pp[0..LENGTH], &fft_domain, level))
        .collect();

    black_box((indet_func, prove_datas));
    return ();
}
