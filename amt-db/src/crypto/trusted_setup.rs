use algebra::{
    AffineCurve, CanonicalDeserialize, CanonicalSerialize, Field, PairingEngine, ProjectiveCurve,
    SerializationError, UniformRand,
};
use rand;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::MulAssign;

use super::{
    paring_provider::{Fr, G1Aff, G2Aff, G1, G2},
    utils::ALLOW_RECOMPUTE,
};

#[derive(CanonicalDeserialize, CanonicalSerialize)]
pub struct PP<PE: PairingEngine>(Vec<G1Aff<PE>>, Vec<G2Aff<PE>>);

impl<PE: PairingEngine> PP<PE> {
    pub fn trusted_setup(tau: Option<Fr<PE>>, depth: usize) -> PP<PE> {
        let random_tau = Fr::<PE>::rand(&mut rand::thread_rng());
        let tau = tau.unwrap_or(random_tau);

        let mut gen1 = G1Aff::<PE>::prime_subgroup_generator().into_projective();
        let gen2 = G2Aff::<PE>::prime_subgroup_generator().into_projective();

        let mut g1pp: Vec<G1Aff<PE>> = vec![];
        g1pp.reserve(1 << depth);
        for _ in 0..1 << depth {
            g1pp.push(gen1.into_affine());
            gen1.mul_assign(tau.clone());
        }

        let mut g2pp: Vec<G2Aff<PE>> = vec![];
        let mut e = tau.clone();
        g2pp.reserve(depth + 1);
        for _ in 0..depth {
            let value: G2<PE> = gen2.mul(e.clone());
            g2pp.push(value.into_affine());
            e.square_in_place();
        }

        return PP(g1pp, g2pp);
    }

    pub fn from_file(file: &str, expected_depth: usize) -> Result<PP<PE>, error::Error> {
        let buffer = File::open(file)?;
        let pp: PP<PE> = CanonicalDeserialize::deserialize_unchecked(buffer)?;
        let (g1_len, g2_len) = (pp.0.len(), pp.1.len());
        if g1_len != 1 << g2_len {
            Err(error::ErrorKind::InconsistentLength.into())
        } else if expected_depth > g2_len {
            Err(error::ErrorKind::InconsistentLength.into())
        } else if expected_depth < g2_len {
            let g1_vec = pp.0[..1 << expected_depth].to_vec();
            let g2_vec = pp.1[..expected_depth].to_vec();
            Ok(PP(g1_vec, g2_vec))
        } else {
            Ok(pp)
        }
    }

    pub fn from_file_or_new(file: &str, expected_depth: usize) -> PP<PE> {
        match Self::from_file(file, expected_depth) {
            Ok(pp) => pp,
            Err(_) if ALLOW_RECOMPUTE => {
                println!("Start to recompute public parameters");
                let pp = Self::trusted_setup(None, expected_depth);
                let buffer = File::create(file).unwrap();
                pp.serialize_uncompressed(&buffer).unwrap();
                pp
            }
            _ => panic!("Fail to load public parameters"),
        }
    }

    pub fn into_projective(self) -> (Vec<G1<PE>>, Vec<G2<PE>>) {
        let g1pp = self.0.iter().copied().map(|x| G1::<PE>::from(x)).collect();
        let g2pp = self.1.iter().copied().map(|x| G2::<PE>::from(x)).collect();
        (g1pp, g2pp)
    }
}

#[test]
fn test_partial_load() {
    type Pairing = super::paring_provider::Pairing;

    let tau = Fr::<Pairing>::rand(&mut rand::thread_rng());
    let large_pp = PP::<Pairing>::trusted_setup(Some(tau), 8);
    let small_pp = PP::<Pairing>::trusted_setup(Some(tau), 4);

    assert_eq!(small_pp.0[..], large_pp.0[..(small_pp.0.len())]);
    assert_eq!(small_pp.1[..], large_pp.1[..(small_pp.1.len())]);
}

mod error {
    error_chain! {
        links {
        }

        foreign_links {
            File(std::io::Error);
            Serialize(algebra_core::serialize::SerializationError);
        }

        errors {
            InconsistentLength {
                description("In consistent length between expected params and real params")
                display("In consistent length between expected params and real params")
            }
        }
    }
}
