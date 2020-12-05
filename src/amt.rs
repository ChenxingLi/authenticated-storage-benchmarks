type FrBigInt<PE> = <<PE as PairingEngine>::Fr as PrimeField>::BigInt;
type AMTProof<PE> = [AMTNode<PE>; DEPTHS];
type G1<PE> = <PE as PairingEngine>::G1Projective;
type G1Aff<PE> = <PE as PairingEngine>::G1Affine;
type G2<PE> = <PE as PairingEngine>::G2Projective;

#[derive(Clone, Copy)]
struct AMTNode<PE: PairingEngine> {
    commitment: G1<PE>,
    proof: G1<PE>,
}
type CompressedAMTNode<PE> = (G1Aff<PE>, G1Aff<PE>);

impl<PE: PairingEngine> Default for AMTNode<PE> {
    fn default() -> Self {
        Self {
            commitment: G1::<PE>::default(),
            proof: G1::<PE>::default(),
        }
    }
}

impl<PE: PairingEngine> From<CompressedAMTNode<PE>> for AMTNode<PE> {
    fn from((commitment, proof): CompressedAMTNode<PE>) -> Self {
        Self {
            commitment: G1::<PE>::from(commitment),
            proof: G1::<PE>::from(proof),
        }
    }
}

impl<PE: PairingEngine> Into<CompressedAMTNode<PE>> for AMTNode<PE> {
    fn into(self) -> CompressedAMTNode<PE> {
        (self.commitment.into(), self.proof.into())
    }
}

impl<PE: PairingEngine> AMTNode<PE> {
    fn inc(&mut self, commitment: &G1<PE>, proof: &G1<PE>) {
        self.commitment += commitment;
        self.proof += proof;
    }
}

// TODO: this is only an ad-hoc fix to make AMTNode Serializable.

impl<PE: PairingEngine> CanonicalDeserialize for AMTNode<PE> {
    fn deserialize<R: Read>(mut reader: R) -> Result<Self, SerializationError> {
        let compressed_node: CompressedAMTNode<PE> =
            CanonicalDeserialize::deserialize_unchecked(&mut reader)?;
        Ok(compressed_node.into())
    }
}

impl<PE: PairingEngine> CanonicalSerialize for AMTNode<PE> {
    fn serialize<W: Write>(&self, mut writer: W) -> Result<(), SerializationError> {
        let compressed_node: CompressedAMTNode<PE> = self.clone().into();
        compressed_node.serialize_unchecked(&mut writer)
    }

    fn serialized_size(&self) -> usize {
        let compressed_node: CompressedAMTNode<PE> = self.clone().into();
        compressed_node.uncompressed_size()
    }
}

struct AMTree<PE: PairingEngine> {
    data: Vec<PE::Fr>,
    inner_nodes: TreeAccess<AMTNode<PE>, FlattenLayout>,
    commitment: G1<PE>,
}

impl<PE: PairingEngine> AMTree<PE> {
    fn new(name: String, db: KvdbRocksdb) -> Self {
        Self {
            data: vec![PE::Fr::zero(); LENGTH],
            inner_nodes: TreeAccess::new(name, DEPTHS, db),
            commitment: G1::<PE>::default(),
        }
    }

    fn get(&self, index: usize) -> &PE::Fr {
        assert!(index < LENGTH);
        &self.data[index]
    }

    fn inc<PP, I>(&mut self, index: usize, inc_value: I, pp: &PP)
    where
        PP: AMTParams<PE>,
        I: Into<FrBigInt<PE>>,
    {
        assert!(index < LENGTH);
        let value: FrBigInt<PE> = inc_value.into();

        self.data[index] += &<PE::Fr as From<FrBigInt<PE>>>::from(value);

        let leaf_index = bitreverse(index, DEPTHS);
        let node_index = NodeIndex::new(DEPTHS, leaf_index);

        let inc_value = pp.get_idents(index).mul(value);

        self.commitment += &inc_value;

        // Update proof
        for (height, depth) in (0..DEPTHS).map(|height| (height, DEPTHS - height)) {
            let visit_node_index = node_index.to_ancestor(height);
            let proof = pp.get_quotient(depth, index).mul(value);
            self.inner_nodes
                .entry(&visit_node_index)
                .inc(&inc_value, &proof);
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

    fn prove(&mut self, index: usize) -> AMTProof<PE> {
        let leaf_index = bitreverse(index, DEPTHS);
        let node_index = NodeIndex::new(DEPTHS, leaf_index);

        let mut answers = AMTProof::<PE>::default();

        for visit_depth in (1..=DEPTHS).rev() {
            let visit_height = DEPTHS - visit_depth;
            let sibling_node_index = node_index.to_ancestor(visit_height).to_sibling();

            answers[visit_depth - 1] = self.inner_nodes.entry(&sibling_node_index).clone();
        }
        answers
    }

    fn verify<PP>(
        index: usize,
        value: PE::Fr,
        commitment: &G1<PE>,
        proof: AMTProof<PE>,
        pp: &PP,
    ) -> bool
    where
        PP: AMTParams<PE>,
    {
        assert!(index < LENGTH);
        let self_indent = pp.get_idents(index).mul(value);
        let others: PE::G1Projective = proof.iter().map(|node| node.commitment).sum();

        let w_inv = pp.w_inv();
        let g2 = pp.g2();

        if *commitment != self_indent + &others {
            println!(
                "Commitment check fail {},{},{}",
                self_indent.is_zero(),
                others.is_zero(),
                commitment.is_zero()
            );
            return false;
        }

        let tau_pow = |height: usize| pp.get_g2_pow_tau(height);
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
fn test_all<PE: PairingEngine, PP>(amt: &mut AMTree<PE>, public_parameter: &PP, task: &str)
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
    let db = crate::db::open_db("./__test_amt", 0u32);

    let mut amt = AMTree::<Bls12_381>::new("test".to_string(), db);
    let pp: &Bls12_381_AMTPP = &PUBLIC_PARAMETERS;
    test_all(&mut amt, pp, "Empty");

    amt.inc(0, Fr::one(), pp);
    test_all(&mut amt, pp, "one-hot");

    amt.inc(0, Fr::one(), pp);
    amt.inc(LENGTH / 2, Fr::one(), pp);
    test_all(&mut amt, pp, "sibling pair");

    ::std::fs::remove_dir_all("./__test_amt").unwrap();
}

use super::{
    complete_tree::{FlattenCompleteTree, FlattenLayout, NodeIndex, TreeAccess},
    public_parameters::{AMTParams, Bls12_381_AMTPP, PUBLIC_PARAMETERS},
    utils::{bitreverse, DEPTHS, IDX_MASK, LENGTH},
};
use algebra::{
    bls12_381::{Bls12_381, Fr, G1Affine, G1Projective, G2Affine, G2Projective},
    AffineCurve, BigInteger, BigInteger256, CanonicalDeserialize, CanonicalSerialize, FftField,
    Field, FpParameters, One, PairingEngine, PrimeField, ProjectiveCurve, SerializationError, Zero,
};
use cfx_storage::KvdbRocksdb;
use std::convert::From;
use std::default::Default;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::{Add, Index, IndexMut, MulAssign};
