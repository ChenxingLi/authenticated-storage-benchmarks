type FrBigInt<PE> = <<PE as PairingEngine>::Fr as PrimeField>::BigInt;
type AMTProof<PE> = [AMTNode<PE>; DEPTHS];
type G1<PE> = <PE as PairingEngine>::G1Projective;
type G2<PE> = <PE as PairingEngine>::G2Projective;

#[derive(Clone, Copy)]
struct AMTNode<PE: PairingEngine> {
    commitment: G1<PE>,
    proof: G1<PE>,
}

impl<PE: PairingEngine> Default for AMTNode<PE> {
    fn default() -> Self {
        Self {
            commitment: G1::<PE>::default(),
            proof: G1::<PE>::default(),
        }
    }
}

impl<PE: PairingEngine> AMTNode<PE> {
    fn inc(&mut self, comm: &G1<PE>, proof: &G1<PE>) {
        self.commitment += comm;
        self.proof += proof;
    }
}

struct AMTree<PE: PairingEngine> {
    data: Vec<PE::Fr>,
    inner_node: FlattenCompleteTree<AMTNode<PE>>,
    commitment: G1<PE>,
}

impl<PE: PairingEngine> AMTree<PE> {
    fn new() -> Self {
        Self {
            data: vec![PE::Fr::zero(); LENGTH],
            inner_node: FlattenCompleteTree::<AMTNode<PE>>::new(DEPTHS),
            commitment: G1::<PE>::default(),
        }
    }

    fn get(&self, index: usize) -> &PE::Fr {
        assert!(index < LENGTH);
        &self.data[index]
    }

    fn inc<PP, I>(&mut self, index: usize, inc_value: I, public_param: &PP)
    where
        PP: AMTParams<PE>,
        I: Into<FrBigInt<PE>>,
    {
        assert!(index < LENGTH);
        let value: FrBigInt<PE> = inc_value.into();

        self.data[index] += &<PE::Fr as From<FrBigInt<PE>>>::from(value);

        let leaf_index = bitreverse(index, DEPTHS);
        let node_index = NodeIndex::new(DEPTHS, leaf_index);

        let inc_value = public_param.get_idents(index).mul(value);
        self.commitment += &inc_value;

        // Update proof
        for visit_depth in (1..=DEPTHS).rev() {
            let visit_node_index = node_index.to_ancestor(DEPTHS - visit_depth);
            let proof = public_param.get_quotient(visit_depth, index).mul(value);
            self.inner_node[visit_node_index].inc(&inc_value, &proof);
        }
    }

    fn set<PP>(&mut self, index: usize, value: &PE::Fr, public_param: &PP)
    where
        PP: AMTParams<PE>,
    {
        assert!(index < LENGTH);
        let inc_value: FrBigInt<PE> = (self.data[index] - value).into();
        self.inc(index, inc_value, public_param)
    }

    fn commitment(&self) -> &PE::G1Projective {
        return &self.commitment;
    }

    fn prove(&self, index: usize) -> AMTProof<PE> {
        let leaf_index = bitreverse(index, DEPTHS);
        let node_index = NodeIndex::new(DEPTHS, leaf_index);

        let mut answers = AMTProof::<PE>::default();

        for visit_depth in (1..=DEPTHS).rev() {
            let visit_height = DEPTHS - visit_depth;
            let sibling_node_index = node_index.to_ancestor(visit_height).to_sibling();
            // let sibling_node_index = (visit_depth, (tree_index >> visit_height) ^ 1);

            answers[visit_depth - 1] = self.inner_node[sibling_node_index];
        }
        answers
    }

    fn verify<PP>(
        index: usize,
        value: PE::Fr,
        commitment: &G1<PE>,
        proof: AMTProof<PE>,
        public_parameter: &PP,
    ) -> bool
    where
        PP: AMTParams<PE>,
    {
        assert!(index < LENGTH);
        let self_indent = public_parameter.get_idents(index).mul(value);
        let others: PE::G1Projective = proof.iter().map(|node| node.commitment).sum();

        let w_inv = public_parameter.w_inv();
        let g2 = public_parameter.g2();

        if *commitment != self_indent + &others {
            println!(
                "Commitment check fail {},{},{}",
                self_indent.is_zero(),
                others.is_zero(),
                commitment.is_zero()
            );
            return false;
        }

        let tau_pow = |height: usize| public_parameter.get_g2_pow_tau(height);
        let w_pow = |height: usize| g2.mul(w_inv.pow([(index << height & IDX_MASK) as u64]));

        for (index, node) in proof.iter().copied().enumerate() {
            let height = DEPTHS - index - 1;
            if PE::pairing(node.commitment, g2)
                != PE::pairing(node.proof, tau_pow(height) - &w_pow(height))
            {
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
    PP: AMTParams<PE>,
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

use super::{
    complete_tree::{FlattenCompleteTree, NodeIndex},
    public_parameters::{AMTParams, Bls12_381_AMTPP, PUBLIC_PARAMETERS},
    utils::{bitreverse, DEPTHS, IDX_MASK, LENGTH},
};
use algebra::{
    bls12_381::{Bls12_381, Fr, G1Affine, G1Projective, G2Affine, G2Projective},
    AffineCurve, BigInteger, BigInteger256, CanonicalDeserialize, CanonicalSerialize, FftField,
    Field, FpParameters, One, PairingEngine, PrimeField, ProjectiveCurve, SerializationError, Zero,
};
use std::convert::From;
use std::default::Default;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::{Add, Index, IndexMut, MulAssign};
