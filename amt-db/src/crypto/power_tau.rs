use crate::crypto::export::{
    AffineCurve, CanonicalDeserialize, CanonicalSerialize, Field, Fr, G1Aff, G2Aff, PairingEngine,
    ProjectiveCurve, SerializationError, UniformRand, G1, G2,
};
use crate::crypto::pp_file_name;
use rand;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::MulAssign;

#[derive(CanonicalDeserialize, CanonicalSerialize)]
pub struct PowerTau<PE: PairingEngine>(pub Vec<G1Aff<PE>>, pub Vec<G2Aff<PE>>);

impl<PE: PairingEngine> PowerTau<PE> {
    #[cfg(test)]
    fn setup_with_tau(tau: Fr<PE>, depth: usize) -> PowerTau<PE> {
        Self::setup_inner(Some(tau), depth)
    }

    pub fn setup(depth: usize) -> PowerTau<PE> {
        Self::setup_inner(None, depth)
    }

    fn setup_inner(tau: Option<Fr<PE>>, depth: usize) -> PowerTau<PE> {
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
            let value: G2<PE> = gen2.mul(e.clone().into());
            g2pp.push(value.into_affine());
            e.square_in_place();
        }

        return PowerTau(g1pp, g2pp);
    }

    fn from_dir_inner(file: &str, expected_depth: usize) -> Result<PowerTau<PE>, error::Error> {
        let buffer = File::open(file)?;
        let pp: PowerTau<PE> = CanonicalDeserialize::deserialize_unchecked(buffer)?;
        let (g1_len, g2_len) = (pp.0.len(), pp.1.len());
        if g1_len != 1 << g2_len {
            Err(error::ErrorKind::InconsistentLength.into())
        } else if expected_depth > g2_len {
            Err(error::ErrorKind::InconsistentLength.into())
        } else if expected_depth < g2_len {
            let g1_vec = pp.0[..1 << expected_depth].to_vec();
            let g2_vec = pp.1[..expected_depth].to_vec();
            Ok(PowerTau(g1_vec, g2_vec))
        } else {
            Ok(pp)
        }
    }

    pub fn from_dir(dir: &str, expected_depth: usize) -> PowerTau<PE> {
        let file = &format!("{}/{}", dir, pp_file_name::<PE>(expected_depth));
        Self::from_dir_inner(file, expected_depth).expect(&format!(
            "Fail to load public parameters for {} at depth {}, read TODO to generate",
            std::any::type_name::<PE>(),
            expected_depth
        ))
    }

    #[cfg(test)]
    pub fn from_dir_or_new(dir: &str, expected_depth: usize) -> PowerTau<PE> {
        let file = &format!("{}/{}", dir, pp_file_name::<PE>(expected_depth));
        match Self::from_dir_inner(file, expected_depth) {
            Ok(pp) => pp,
            Err(_) => {
                let pp = Self::setup(expected_depth);
                let buffer = File::create(file).unwrap();
                pp.serialize_uncompressed(&buffer).unwrap();
                pp
            }
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
    type Pairing = super::export::Pairing;

    let tau = Fr::<Pairing>::rand(&mut rand::thread_rng());
    let large_pp = PowerTau::<Pairing>::setup_with_tau(tau, 8);
    let small_pp = PowerTau::<Pairing>::setup_with_tau(tau, 4);

    assert_eq!(small_pp.0[..], large_pp.0[..(small_pp.0.len())]);
    assert_eq!(small_pp.1[..], large_pp.1[..(small_pp.1.len())]);
}

mod error {
    error_chain! {
        links {
        }

        foreign_links {
            File(std::io::Error);
            Serialize(crate::crypto::export::SerializationError);
        }

        errors {
            InconsistentLength {
                description("In consistent length between expected params and real params")
                display("In consistent length between expected params and real params")
            }
        }
    }
}
