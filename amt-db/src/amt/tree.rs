use super::{
    node::{AMTNode, NodeIndex},
    paring_provider::{Fr, FrInt, G1},
    prove_params::AMTParams,
    utils::bitreverse,
};
use crate::storage::{KvdbRocksdb, LayoutTrait, TreeAccess};
use algebra::{
    BigInteger, CanonicalDeserialize, CanonicalSerialize, Field, PairingEngine, PrimeField,
    ProjectiveCurve, Zero,
};
use static_assertions::_core::ops::{Deref, DerefMut};
use std::sync::Arc;

pub type AMTProof<PE> = Vec<AMTNode<PE>>;

pub trait AMTConfigTrait {
    type PE: PairingEngine;
    type Name: Into<Vec<u8>> + Clone;
    type Data: AMTData<Fr<Self::PE>> + Default + Clone + CanonicalSerialize + CanonicalDeserialize;
    type DataLayout: LayoutTrait<usize>;
    type TreeLayout: LayoutTrait<NodeIndex>;

    const DEPTHS: usize;
    const LENGTH: usize = 1 << Self::DEPTHS;
    const IDX_MASK: usize = Self::LENGTH - 1;
}

pub trait AMTData<P: PrimeField> {
    fn as_fr_int(&self) -> P::BigInt;
    fn as_fr(&self) -> P {
        self.as_fr_int().into()
    }
}

pub struct AMTNodeWriteGuard<'a, C: AMTConfigTrait> {
    index: usize,
    value: C::Data,
    old_fr_int: FrInt<C::PE>,
    tree: &'a mut AMTree<C>,
}

impl<'a, C: AMTConfigTrait> Deref for AMTNodeWriteGuard<'a, C> {
    type Target = C::Data;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, C: AMTConfigTrait> DerefMut for AMTNodeWriteGuard<'a, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<'a, C: AMTConfigTrait> Drop for AMTNodeWriteGuard<'a, C> {
    fn drop(&mut self) {
        let mut fr_int = self.value.as_fr_int();
        fr_int.sub_noborrow(&self.old_fr_int);
        std::mem::swap(self.tree.get_mut(self.index), &mut self.value);
        self.tree.update(self.index, fr_int);
    }
}

#[derive(Clone)]
pub struct AMTree<C: AMTConfigTrait> {
    db: KvdbRocksdb,
    name: C::Name,
    data: TreeAccess<usize, C::Data, C::DataLayout>,
    inner_nodes: TreeAccess<NodeIndex, AMTNode<C::PE>, C::TreeLayout>,
    pp: Arc<AMTParams<C::PE>>,

    dirty: bool,
}

impl<C: AMTConfigTrait> AMTree<C> {
    pub fn new(name: C::Name, db: KvdbRocksdb, pp: Arc<AMTParams<C::PE>>) -> Self {
        let name_with_prefix = |mut prefix: Vec<u8>| {
            prefix.extend_from_slice(&name.clone().into());
            prefix
        };
        Self {
            data: TreeAccess::new(name_with_prefix(vec![0u8]), db.clone()),
            inner_nodes: TreeAccess::new(name_with_prefix(vec![1u8]), db.clone()),
            db,
            name,
            dirty: false,
            pp,
        }
    }

    // Because the underlying data has a cache, so most read operation requires a mutable ref.
    pub fn get(&mut self, index: usize) -> &C::Data {
        assert!(index < C::LENGTH);
        self.data.get(&index)
    }

    fn get_mut(&mut self, index: usize) -> &mut C::Data {
        assert!(index < C::LENGTH);
        self.data.get_mut(&index)
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn write(&mut self, index: usize) -> AMTNodeWriteGuard<C> {
        let value = std::mem::take(self.data.get_mut(&index));
        let old_fr_int = value.as_fr_int();
        AMTNodeWriteGuard {
            index,
            value,
            old_fr_int,
            tree: self,
        }
    }

    pub fn commitment(&mut self) -> &G1<C::PE> {
        return &self.inner_nodes.get(&NodeIndex::root(C::DEPTHS)).commitment;
    }

    pub fn flush(&mut self) {
        self.data.flush();
        self.inner_nodes.flush();
    }

    pub fn update(&mut self, index: usize, update_fr_int: FrInt<C::PE>) {
        assert!(index < C::LENGTH);

        if update_fr_int == FrInt::<C::PE>::from(0) {
            return;
        }

        self.dirty = true;

        let update_fr: Fr<C::PE> = update_fr_int.into();

        let inc_comm = self.pp.get_idents(index).mul(update_fr);

        // Update proof
        self.inner_nodes
            .get_mut(&NodeIndex::root(C::DEPTHS))
            .commitment += &inc_comm;

        let leaf_index = bitreverse(index, C::DEPTHS);
        let node_index = NodeIndex::new(C::DEPTHS, leaf_index, C::DEPTHS);

        for (height, depth) in (0..C::DEPTHS).map(|height| (height, C::DEPTHS - height)) {
            let visit_node_index = node_index.to_ancestor(height);
            let proof = self.pp.get_quotient(depth, index).mul(update_fr);
            self.inner_nodes
                .get_mut(&visit_node_index)
                .inc(&inc_comm, &proof);
        }
    }

    pub fn prove(&mut self, index: usize) -> AMTProof<C::PE> {
        let leaf_index = bitreverse(index, C::DEPTHS);
        let node_index = NodeIndex::new(C::DEPTHS, leaf_index, C::DEPTHS);

        let mut answers = vec![Default::default(); C::DEPTHS];

        for visit_depth in (1..=C::DEPTHS).rev() {
            let visit_height = C::DEPTHS - visit_depth;
            let sibling_node_index = node_index.to_ancestor(visit_height).to_sibling();

            answers[visit_depth - 1] = self.inner_nodes.get_mut(&sibling_node_index).clone();
        }
        answers
    }

    pub fn verify(
        index: usize,
        value: Fr<C::PE>,
        commitment: &G1<C::PE>,
        proof: AMTProof<C::PE>,
        pp: &AMTParams<C::PE>,
    ) -> bool {
        assert!(index < C::LENGTH);
        let self_indent = pp.get_idents(index).mul(value);
        let others: G1<C::PE> = proof.iter().map(|node| node.commitment).sum();

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
        let w_pow = |height: usize| g2.mul(w_inv.pow([(index << height & C::IDX_MASK) as u64]));

        for (index, node) in proof.iter().copied().enumerate() {
            let height = C::DEPTHS - index - 1;
            if C::PE::pairing(node.commitment, g2)
                != C::PE::pairing(node.proof, tau_pow(height) - &w_pow(height))
            {
                println!("Pairing check fails at height {}", height);
                return false;
            }
        }
        return true;
    }
}
