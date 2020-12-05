use super::utils::{ALLOW_RECOMPUTE, DEPTHS};
use algebra::bls12_381::{Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use algebra::{
    AffineCurve, CanonicalDeserialize, CanonicalSerialize, Field, ProjectiveCurve,
    SerializationError, UniformRand,
};
use rand;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::MulAssign;

#[derive(CanonicalDeserialize, CanonicalSerialize)]
pub struct PP(Vec<G1Affine>, Vec<G2Affine>);

impl PP {
    pub fn trusted_setup(tau: Fr, depth: usize) -> PP {
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
        // g2pp.push(gen2.into_affine());
        for _ in 0..depth {
            let value: G2Projective = gen2.mul(e.clone());
            g2pp.push(value.into_affine());
            e.square_in_place();
        }

        return PP(g1pp, g2pp);
    }

    pub fn load_pp(file: &str) -> Result<PP, error::Error> {
        let buffer = File::open(file)?;
        let pp: PP = CanonicalDeserialize::deserialize_unchecked(buffer)?;
        return Ok(pp);
    }

    pub fn load_or_create_pp(file: &str) -> PP {
        match Self::load_pp(file) {
            Ok(pp) => pp,
            Err(_) if ALLOW_RECOMPUTE => {
                println!("Start to recompute public parameters");
                let tau: Fr = Fr::rand(&mut rand::thread_rng());
                let pp = Self::trusted_setup(tau, DEPTHS);
                let buffer = File::create(file).unwrap();
                pp.serialize_uncompressed(&buffer).unwrap();
                pp
            }
            _ => panic!("Fail to load public parameters"),
        }
    }

    pub fn into_projective(self) -> (Vec<G1Projective>, Vec<G2Projective>) {
        let g1pp: Vec<G1Projective> = self
            .0
            .iter()
            .copied()
            .map(|x| G1Projective::from(x))
            .collect();

        let g2pp: Vec<G2Projective> = self
            .1
            .iter()
            .copied()
            .map(|x| G2Projective::from(x))
            .collect();
        (g1pp, g2pp)
    }
}

mod error {
    error_chain! {
        links {
        }

        foreign_links {
            File(std::io::Error);
            Serialize(algebra_core::serialize::SerializationError);
        }
    }
}
