use crate::crypto::export::{
    k_adicity, AffineCurve, CanonicalDeserialize, CanonicalSerialize, EvaluationDomain, FftField,
    Field, Fr, G2Aff, PairingEngine, Radix2EvaluationDomain, SerializationError, Zero, G1, G2,
};
use crate::crypto::power_tau::PowerTau;
use std::io::{Read, Write};

#[derive(CanonicalDeserialize, CanonicalSerialize)]
pub struct AMTParams<PE: PairingEngine> {
    indents: Vec<G1<PE>>,
    quotients: Vec<Vec<G1<PE>>>,
    g2pp: Vec<G2<PE>>,
    g2: G2<PE>,
    w_inv: Fr<PE>,
}

impl<PE: PairingEngine> AMTParams<PE> {
    pub fn get_idents(&self, index: usize) -> &G1<PE> {
        &self.indents[index]
    }

    pub fn get_quotient(&self, depth: usize, index: usize) -> &G1<PE> {
        &self.quotients[depth - 1][index]
    }

    pub fn get_g2_pow_tau(&self, height: usize) -> &G2<PE> {
        &self.g2pp[height]
    }

    pub fn g2(&self) -> G2<PE> {
        self.g2.clone()
    }

    pub fn w_inv(&self) -> Fr<PE> {
        self.w_inv.clone()
    }

    pub fn from_pp(pp: PowerTau<PE>, depth: usize) -> Self {
        let (g1pp, g2pp) = pp.into_projective();

        let length: usize = 1 << depth;

        assert_eq!(g1pp.len(), length);
        assert_eq!(g2pp.len(), depth);

        let fft_domain = Radix2EvaluationDomain::<Fr<PE>>::new(length).unwrap();

        let indents: Vec<G1<PE>> = fft_domain.fft(&g1pp[0..length]);

        let quotients: Vec<Vec<G1<PE>>> = (1..=depth)
            .map(|d| Self::gen_quotients(&g1pp[0..length], &fft_domain, d))
            .collect();

        let w_inv = Fr::<PE>::get_root_of_unity(length)
            .unwrap()
            .inverse()
            .unwrap();

        let g2 = G2Aff::<PE>::prime_subgroup_generator().into_projective();

        Self {
            indents,
            quotients,
            g2pp,
            g2,
            w_inv,
        }
    }

    fn gen_quotients(
        g1pp: &[G1<PE>],
        fft_domain: &Radix2EvaluationDomain<Fr<PE>>,
        depth: usize,
    ) -> Vec<G1<PE>> {
        assert!(g1pp.len() <= 1 << 32);

        let length = g1pp.len();
        let max_depth = k_adicity(2, length) as usize;

        assert_eq!(1 << max_depth, length);
        assert!(max_depth >= depth);
        assert!(depth >= 1);

        let chunk_length = (1 << (max_depth - depth)) as usize;
        let chunk_num = length / chunk_length;

        let mut g1pp_chunks_iter = g1pp.chunks(1 << (max_depth - depth) as usize);
        let mut coeff = vec![G1::<PE>::zero(); length];

        for i in 0..(chunk_num / 2) {
            coeff[(2 * i + 1) * chunk_length..(2 * i + 2) * chunk_length]
                .copy_from_slice(g1pp_chunks_iter.next().unwrap());
            g1pp_chunks_iter.next();
        }

        return fft_domain.fft(&coeff);
    }
}

#[test]
fn test_ident_prove() {
    const TEST_LEVEL: usize = 6;
    const TEST_LENGTH: usize = 1 << TEST_LEVEL;

    let (g1pp, g2pp) = PowerTau::<Pairing>::from_file_or_new("./pp", TEST_LEVEL).into_projective();

    let w = Fr::<Pairing>::get_root_of_unity(TEST_LENGTH).unwrap();
    let w_inv = w.inverse().unwrap();
    assert_eq!(w.pow(&[TEST_LENGTH as u64]), Fr::<Pairing>::one());

    let fft_domain = Radix2EvaluationDomain::<Fr<Pairing>>::new(TEST_LENGTH).unwrap();
    let indent_func = fft_domain.fft(&g1pp[0..TEST_LENGTH]);

    let g2 = G2Aff::<Pairing>::prime_subgroup_generator().into_projective();

    for depth in 1..=TEST_LEVEL {
        let prove_data =
            AMTParams::<Pairing>::gen_quotients(&g1pp[0..TEST_LENGTH], &fft_domain, depth);
        for i in 0..TEST_LENGTH {
            assert_eq!(
                Pairing::pairing(indent_func[i], g2),
                Pairing::pairing(
                    prove_data[i],
                    g2pp[TEST_LEVEL - depth]
                        + g2.mul::<FrInt<Pairing>>(
                            w_inv.pow([(i * (TEST_LENGTH >> depth)) as u64]).into()
                        ),
                )
            );
        }
    }
}
#[cfg(test)]
use super::export::{FrInt, Pairing};
#[cfg(test)]
use crate::crypto::export::{One, ProjectiveCurve};
