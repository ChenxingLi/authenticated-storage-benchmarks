use super::node::{AMTNode, NodeIndex};
use super::write_guard::AMTNodeWriteGuard;
use crate::crypto::export::{Field, PairingEngine, PrimeField, ProjectiveCurve, Zero};
use crate::crypto::{
    paring_provider::{Fr, FrInt, G1},
    AMTParams, TypeUInt,
};
use crate::storage::{DBAccess, KvdbRocksdb, LayoutTrait, StorageDecodable, StorageEncodable};
use std::sync::Arc;

pub trait AMTConfigTrait {
    type PE: PairingEngine;
    type Name: StorageEncodable;
    type Data: AMTData<Fr<Self::PE>> + Default + Clone + StorageEncodable + StorageDecodable;

    type DataLayout: LayoutTrait<usize>;
    type TreeLayout: LayoutTrait<NodeIndex<Self::Height>>;
    type Height: TypeUInt;

    const DEPTHS: usize = Self::Height::USIZE;
    const LENGTH: usize = 1 << Self::DEPTHS;
    const IDX_MASK: usize = Self::LENGTH - 1;
}

pub trait AMTData<P: PrimeField> {
    fn as_fr_int(&self) -> P::BigInt;
    fn as_fr(&self) -> P {
        self.as_fr_int().into()
    }
}

#[derive(Clone)]
pub struct AMTree<C: AMTConfigTrait> {
    db: KvdbRocksdb,
    name: C::Name,
    data: DBAccess<usize, C::Data, C::DataLayout>,
    inner_nodes: DBAccess<NodeIndex<C::Height>, AMTNode<G1<C::PE>>, C::TreeLayout>,
    pp: Arc<AMTParams<C::PE>>,

    dirty: bool,
}

pub type AMTProof<G> = Vec<AMTNode<G>>;

impl<C: AMTConfigTrait> AMTree<C> {
    pub fn new(name: C::Name, db: KvdbRocksdb, pp: Arc<AMTParams<C::PE>>) -> Self {
        let name_with_prefix = |mut prefix: Vec<u8>| {
            prefix.extend_from_slice(&name.storage_encode());
            prefix
        };
        Self {
            data: DBAccess::new(name_with_prefix(vec![0u8]), db.clone()),
            inner_nodes: DBAccess::new(name_with_prefix(vec![1u8]), db.clone()),
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

    pub(super) fn get_mut(&mut self, index: usize) -> &mut C::Data {
        assert!(index < C::LENGTH);
        self.data.get_mut(&index)
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn write(&mut self, index: usize) -> AMTNodeWriteGuard<C> {
        let value = std::mem::take(self.data.get_mut(&index));
        AMTNodeWriteGuard::new(index, value, self)
    }

    pub fn commitment(&mut self) -> &G1<C::PE> {
        &self.inner_nodes.get(&NodeIndex::root()).commitment
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

        let inc_comm = self.pp.get_idents(index).mul(update_fr_int);

        // Update proof
        self.inner_nodes.get_mut(&NodeIndex::root()).commitment += &inc_comm;

        let leaf_index = bitreverse(index, C::DEPTHS);
        let node_index = NodeIndex::new(C::DEPTHS, leaf_index);

        for (height, depth) in (0..C::DEPTHS).map(|height| (height, C::DEPTHS - height)) {
            let visit_node_index = node_index.to_ancestor(height);
            let proof = self.pp.get_quotient(depth, index).mul(update_fr_int);
            let node = self.inner_nodes.get_mut(&visit_node_index);
            node.commitment += &inc_comm;
            node.proof += &proof;
        }
    }

    pub fn prove(&mut self, index: usize) -> AMTProof<G1<C::PE>> {
        let leaf_index = bitreverse(index, C::DEPTHS);
        let node_index = NodeIndex::new(C::DEPTHS, leaf_index);

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
        proof: AMTProof<G1<C::PE>>,
        pp: &AMTParams<C::PE>,
    ) -> bool {
        assert!(index < C::LENGTH);
        let self_indent = pp.get_idents(index).mul(value.into());
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
        let w_pow =
            |height: usize| g2.mul(w_inv.pow([(index << height & C::IDX_MASK) as u64]).into());

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

#[inline]
fn bitreverse(mut n: usize, l: usize) -> usize {
    let mut r = 0;
    for _ in 0..l {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}
