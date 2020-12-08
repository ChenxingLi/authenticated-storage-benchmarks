pub mod db_access;
pub mod layout;
pub mod node;
pub mod paring_provider;
pub mod prove_params;
pub mod trusted_setup;
pub mod utils;

use self::{
    db_access::TreeAccess,
    layout::{FlattenArray, FlattenTree, NodeIndex},
    node::AMTNode,
    paring_provider::{Fr, FrInt, G1},
    prove_params::AMTParams,
    utils::*,
};
use algebra::{Field, PairingEngine, ProjectiveCurve, Zero};
use cfx_storage::KvdbRocksdb;
use db::SystemDB;
use std::sync::Arc;

#[cfg(test)]
mod test;

pub type AMTProof<PE> = [AMTNode<PE>; DEPTHS];

pub struct AMTree<PE: PairingEngine> {
    data: TreeAccess<Fr<PE>, FlattenArray>,
    inner_nodes: TreeAccess<AMTNode<PE>, FlattenTree>,
    commitment: G1<PE>,
}

impl<PE: PairingEngine> AMTree<PE> {
    fn new(name: String, db: Arc<SystemDB>) -> Self {
        let kvdb = db.key_value().clone();
        Self {
            data: TreeAccess::new(
                format!("data:{}", name),
                KvdbRocksdb {
                    kvdb: kvdb.clone(),
                    col: 0,
                },
            ),
            inner_nodes: TreeAccess::new(format!("tree:{}", name), KvdbRocksdb { kvdb, col: 0 }),
            commitment: G1::<PE>::default(),
        }
    }

    fn get(&mut self, index: usize) -> &PE::Fr {
        assert!(index < LENGTH);
        self.data.entry(&index)
    }

    fn inc<I>(&mut self, index: usize, inc_value: I, pp: &AMTParams<PE>)
    where
        I: Into<FrInt<PE>>,
    {
        assert!(index < LENGTH);
        let value: FrInt<PE> = inc_value.into();

        *self.data.entry(&index) += &<PE::Fr as From<FrInt<PE>>>::from(value);

        let leaf_index = bitreverse(index, DEPTHS);
        let node_index = NodeIndex::new(DEPTHS, leaf_index, DEPTHS);

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

    fn set(&mut self, index: usize, value: &Fr<PE>, public_param: &AMTParams<PE>) {
        assert!(index < LENGTH);
        let inc_value: FrInt<PE> = (*self.data.entry(&index) - value).into();
        self.inc(index, inc_value, public_param)
    }

    fn commitment(&self) -> &PE::G1Projective {
        return &self.commitment;
    }

    fn prove(&mut self, index: usize) -> AMTProof<PE> {
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

    fn verify(
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
