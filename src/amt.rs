// use algebra::bls12_381::{
//     g1, g2, Bls12_381, Fq, Fq12, Fq2, Fr, G1Affine, G1Projective, G2Affine, G2Projective,
// };

use super::{
    complete_tree::{FlattenCompleteTree, ROOT_INDEX},
    public_parameters::{Bls12_381_AMTPP, AMTPP, PUBLIC_PARAMETERS},
    utils::{bitreverse, DEPTHS, IDX_MASK, LENGTH},
};
use algebra::{
    bls12_381::{Bls12_381, Fr, G1Affine, G1Projective, G2Affine, G2Projective},
    AffineCurve, BigInteger, BigInteger256, CanonicalDeserialize, CanonicalSerialize, FftField,
    Field, FpParameters, One, PairingEngine, PrimeField, ProjectiveCurve, Zero,
};
use std::convert::From;
use std::fs::File;
use std::ops::{Add, Index, IndexMut, MulAssign};

type FrBigInt<PE> = <<PE as PairingEngine>::Fr as PrimeField>::BigInt;
type AMTProof<PE> = [[<PE as PairingEngine>::G1Projective; 2]; DEPTHS];

struct AMTree<PE>
where
    PE: PairingEngine,
{
    data: Vec<PE::Fr>,
    inner_node: FlattenCompleteTree<PE::G1Projective>,
    inner_proof: FlattenCompleteTree<PE::G1Projective>,
}

impl<PE: PairingEngine> AMTree<PE> {
    fn new() -> Self {
        Self {
            data: vec![PE::Fr::zero(); LENGTH],
            inner_node: FlattenCompleteTree::<PE::G1Projective>::new(DEPTHS),
            inner_proof: FlattenCompleteTree::<PE::G1Projective>::new(DEPTHS),
        }
    }

    fn get(&self, index: usize) -> &PE::Fr {
        assert!(index < LENGTH);
        &self.data[index]
    }

    fn inc<PP, I>(&mut self, index: usize, inc_value: I, public_param: &PP)
    where
        PP: AMTPP<PE>,
        I: Into<FrBigInt<PE>>,
    {
        assert!(index < LENGTH);
        let value: FrBigInt<PE> = inc_value.into();

        self.data[index] += &<PE::Fr as From<FrBigInt<PE>>>::from(value);

        let tree_index = bitreverse(index, DEPTHS);
        let inc_value = public_param.get_idents()[index].mul(value);

        for visit_depth in (1..=DEPTHS).rev() {
            let visit_height = DEPTHS - visit_depth;
            let visit_node_index = (visit_depth, tree_index >> visit_height);

            self.inner_node[visit_node_index] += &inc_value;
            self.inner_proof[visit_node_index] +=
                &public_param.get_prove_cache()[visit_depth - 1][index].mul(value);
        }
        self.inner_node[ROOT_INDEX] += &inc_value;
    }

    fn set<PP>(&mut self, index: usize, value: &PE::Fr, public_param: &PP)
    where
        PP: AMTPP<PE>,
    {
        assert!(index < LENGTH);
        let inc_value: FrBigInt<PE> = (self.data[index] - value).into();
        self.inc(index, inc_value, public_param)
    }

    fn commitment(&self) -> &PE::G1Projective {
        return &self.inner_node[ROOT_INDEX];
    }

    fn prove(&self, index: usize) -> AMTProof<PE> {
        let tree_index = bitreverse(index, DEPTHS);
        let mut answers = AMTProof::<PE>::default();

        for visit_depth in (1..=DEPTHS).rev() {
            let visit_height = DEPTHS - visit_depth;
            let sibling_node_index = (visit_depth, (tree_index >> visit_height) ^ 1);

            let data = self.inner_node[sibling_node_index];
            let proof = self.inner_proof[sibling_node_index];

            answers[visit_depth - 1] = [data, proof];
        }
        answers
    }

    fn verify<PP>(
        index: usize,
        value: PE::Fr,
        commitment: &PE::G1Projective,
        proof: AMTProof<PE>,
        public_parameter: &PP,
    ) -> bool
    where
        PP: AMTPP<PE>,
    {
        assert!(index < LENGTH);
        let self_indent = public_parameter.get_idents()[index].mul(value);
        let others: PE::G1Projective = proof.iter().map(|x| x[0]).sum();
        let (g2pp, w_inv) = public_parameter.get_verification();
        let g2 = g2pp[0];

        if *commitment != self_indent + &others {
            println!(
                "Commitment check fail {},{},{}",
                self_indent.is_zero(),
                others.is_zero(),
                commitment.is_zero()
            );
            return false;
        }

        let tau_pow = |height: usize| g2pp[1 + height];
        let w_pow = |height: usize| g2.mul(w_inv.pow([(index << height & IDX_MASK) as u64]));

        for (height, data, proof) in proof
            .iter()
            .copied()
            .enumerate()
            .map(|(index, [data, proof])| (DEPTHS - index - 1, data, proof))
        {
            if PE::pairing(data, g2) != PE::pairing(proof, tau_pow(height) - &w_pow(height)) {
                println!("Pairing check fails at height {}", height);
                return false;
            }
        }
        return true;
    }
}

#[cfg(test)]
fn test_all<PE: PairingEngine, PP>(amt: &AMTree<PE>, public_parameter: &PP, task: &str)
where
    PP: AMTPP<PE>,
{
    for i in 0..LENGTH {
        let proof = amt.prove(i);
        let value = amt.get(i);

        assert!(
            AMTree::<PE>::verify(i, *value, amt.commitment(), proof, public_parameter),
            "fail at task {} pos {}",
            task,
            i
        );
    }
}

#[test]
fn test_amt() {
    let mut amt = AMTree::<Bls12_381>::new();
    let pp: &Bls12_381_AMTPP = &PUBLIC_PARAMETERS;
    test_all(&amt, pp, "Empty");

    amt.inc(0, Fr::one(), pp);
    test_all(&amt, pp, "one-hot");

    amt.inc(0, Fr::one(), pp);
    amt.inc(LENGTH / 2, Fr::one(), pp);
    test_all(&amt, pp, "sibling pair");
}
