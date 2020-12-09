use super::{
    node::{AMTNode, NodeIndex},
    paring_provider::{Fr, FrInt, G1},
    prove_params::AMTParams,
    utils::{bitreverse, DEPTHS, IDX_MASK, LENGTH},
};
use crate::storage::{FlattenArray, FlattenTree, KvdbRocksdb, SystemDB, TreeAccess};
use algebra::{
    BigInteger, CanonicalDeserialize, CanonicalSerialize, Field, FpParameters, PairingEngine,
    PrimeField, ProjectiveCurve, Zero,
};
use std::sync::Arc;

pub type AMTProof<PE> = [AMTNode<PE>; DEPTHS];

pub trait AMTData<P: PrimeField> {
    fn as_fr_int(&self) -> P::BigInt;
    fn as_fr(&self) -> P {
        self.as_fr_int().into()
    }
}

pub struct AMTree<PE: PairingEngine, D>
where
    D: AMTData<Fr<PE>> + Default + Clone + CanonicalSerialize + CanonicalDeserialize,
{
    data: TreeAccess<D, FlattenArray>,
    inner_nodes: TreeAccess<AMTNode<PE>, FlattenTree>,
    commitment: G1<PE>,
}

impl<PE: PairingEngine, D> AMTree<PE, D>
where
    D: AMTData<Fr<PE>> + Default + Clone + CanonicalSerialize + CanonicalDeserialize,
{
    pub fn new(name: String, db: Arc<SystemDB>) -> Self {
        Self {
            data: TreeAccess::new(
                format!("data:{}", name),
                KvdbRocksdb {
                    kvdb: db.key_value().clone(),
                    col: 0,
                },
            ),
            inner_nodes: TreeAccess::new(
                format!("tree:{}", name),
                KvdbRocksdb {
                    kvdb: db.key_value().clone(),
                    col: 0,
                },
            ),
            commitment: G1::<PE>::default(),
        }
    }

    pub fn get(&mut self, index: usize) -> &D {
        assert!(index < LENGTH);
        self.data.entry(&index)
    }

    pub fn commitment(&self) -> &PE::G1Projective {
        return &self.commitment;
    }

    pub fn update<F>(&mut self, index: usize, update: F, pp: &AMTParams<PE>)
    where
        F: FnOnce(&mut D),
    {
        assert!(index < LENGTH);
        let item = self.data.entry(&index);

        let old_value: FrInt<PE> = item.as_fr_int();
        update(item);
        let mut new_value: FrInt<PE> = item.as_fr_int();
        assert!(new_value < <Fr::<PE> as PrimeField>::Params::MODULUS);
        let _borrow = new_value.sub_noborrow(&old_value);

        let update_fr: Fr<PE> = new_value.into();
        let inc_value = pp.get_idents(index).mul(update_fr);

        self.commitment += &inc_value;

        // Update proof

        let leaf_index = bitreverse(index, DEPTHS);
        let node_index = NodeIndex::new(DEPTHS, leaf_index, DEPTHS);

        for (height, depth) in (0..DEPTHS).map(|height| (height, DEPTHS - height)) {
            let visit_node_index = node_index.to_ancestor(height);
            let proof = pp.get_quotient(depth, index).mul(update_fr);
            self.inner_nodes
                .entry(&visit_node_index)
                .inc(&inc_value, &proof);
        }
    }

    pub fn prove(&mut self, index: usize) -> AMTProof<PE> {
        let leaf_index = bitreverse(index, DEPTHS);
        let node_index = NodeIndex::new(DEPTHS, leaf_index, DEPTHS);

        let mut answers = AMTProof::<PE>::default();

        for visit_depth in (1..=DEPTHS).rev() {
            let visit_height = DEPTHS - visit_depth;
            let sibling_node_index = node_index.to_ancestor(visit_height).to_sibling();

            answers[visit_depth - 1] = self.inner_nodes.entry(&sibling_node_index).clone();
        }
        answers
    }

    pub fn verify(
        index: usize,
        value: PE::Fr,
        commitment: &G1<PE>,
        proof: AMTProof<PE>,
        pp: &AMTParams<PE>,
    ) -> bool {
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

        let tau_pow = |height: usize| *pp.get_g2_pow_tau(height);
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
