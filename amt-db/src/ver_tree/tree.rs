use super::{Commitment, Key, Tree, TreeName, MAX_VERSION_NUMBER};
use crate::crypto::export::{BigInteger, FrInt, G1Aff, ProjectiveCurve, G1};
use crate::storage::DBColumn;
use crate::ver_tree::node::EpochPosition;
use crate::{
    crypto::{
        export::{CanonicalDeserialize, CanonicalSerialize, Pairing, SerializationError, ToBytes},
        AMTParams,
    },
    impl_storage_from_canonical,
    storage::{StorageDecodable, StorageEncodable},
};
use hashbrown::{HashMap, HashSet};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::Arc;

type NodeIndex = u32;
type TreesLayer = HashMap<Vec<NodeIndex>, TreeWithInfo>;

/// The `VersionTree`
#[derive(Clone)]
pub struct VersionTree {
    root: G1<Pairing>,
    producer: TreeProducer,
    forest: Vec<TreesLayer>,

    fast_mode: bool,
}

impl VersionTree {
    pub fn new(db: DBColumn, pp: Arc<AMTParams<Pairing>>, only_root: bool) -> Self {
        let mut forest = Vec::with_capacity(8);
        forest.push(Default::default());
        Self {
            forest,
            producer: TreeProducer { db, pp, only_root },
            root: Default::default(),
            fast_mode: only_root,
        }
    }

    pub(crate) fn get_tree_mut(&mut self, name: &TreeName) -> &mut Tree {
        let (ancestor_layers, tree_layer) = {
            let level = name.0.len();
            if self.forest.len() < level + 1 {
                self.forest.resize(level + 1, Default::default())
            }
            let (front, end) = self.forest.split_at_mut(level);
            (front, &mut end[0])
        };

        let producer = &self.producer;
        let new_tree = |tree_name: &TreeName| TreeWithInfo {
            tree: producer.new_tree(tree_name),
            mark_in_parent: false,
            children_marks: Default::default(),
        };

        let tree_with_info = tree_layer
            .entry(name.0.clone())
            .or_insert_with(|| new_tree(&name));

        let mut last_tree = &mut tree_with_info.tree;

        if !tree_with_info.mark_in_parent {
            tree_with_info.mark_in_parent = true;
            if name.0.len() == 0 {
                last_tree.set_commitment(&self.root);
            }

            for (level, layer) in ancestor_layers.iter_mut().enumerate().rev() {
                let anc_tree_name = TreeName(name.0[..level].to_vec());
                let sub_index = name.0[level];
                let tree_with_info = layer
                    .entry(anc_tree_name.clone().0)
                    .or_insert_with(|| new_tree(&anc_tree_name));
                tree_with_info.children_marks.insert(sub_index);
                last_tree.set_commitment(&tree_with_info.tree.subtree_root(sub_index as usize));
                last_tree = &mut tree_with_info.tree;

                if tree_with_info.mark_in_parent {
                    break;
                } else {
                    tree_with_info.mark_in_parent = true;
                }
            }
        }
        &mut tree_with_info.tree
    }

    pub fn inc_key_ver(&mut self, key: &Key, version: Option<VerInfo>) -> VerInfo {
        let VerInfo {
            version,
            level,
            slot_index,
        } = match version {
            None => self.allocate_vacant_slot(key),
            Some(ver_info) => ver_info,
        };

        let visit_amt = self.get_tree_mut(&key.tree_at_level(level));
        let node = key.index_at_level(level);
        if visit_amt.only_root() {
            visit_amt.update(node, fr_int_pow_2((slot_index as u32 + 1) * 5));
        } else {
            visit_amt.write_versions(node).key_versions[slot_index as usize] += 1;
        }
        assert!(version < MAX_VERSION_NUMBER);
        return VerInfo {
            version: version + 1,
            level,
            slot_index,
        };
    }

    pub fn allocate_vacant_slot(&mut self, key: &Key) -> VerInfo {
        for level in 0..32 {
            let visit_amt = self.get_tree_mut(&key.tree_at_level(level));
            let node_index = key.index_at_level(level);

            if visit_amt.get(node_index).key_versions.len() < 5 {
                let mut data = visit_amt.write_versions(node_index);
                let slot_index = data.key_versions.len();
                data.key_versions.push(0);
                std::mem::drop(data);

                return VerInfo {
                    version: 0,
                    level,
                    slot_index: slot_index as u8,
                };
            }
        }
        panic!("Exceed maximum support level");
    }

    fn commit_tree(
        name: &TreeName,
        epoch: u64,
        start_pos: u64, //TODO: ugly.
        layers: &mut [TreesLayer],
        updates: &mut SubTreeRootRecorder,
    ) -> (bool, Commitment) {
        let (this_layer, rest_layers) = layers.split_first_mut().unwrap();

        let tree_with_info = this_layer.get_mut(&name.0).unwrap();

        for &index in tree_with_info.children_marks.iter() {
            let child_name = name.child(index);
            let (dirty, commitment) =
                Self::commit_tree(&child_name, epoch, start_pos, rest_layers, updates);
            if dirty {
                *tree_with_info.tree.subtree_root_mut(index as usize) = commitment;
                let mut node = tree_with_info.tree.write_versions(index as usize);
                node.tree_version += 1;
                node.tree_position = EpochPosition {
                    epoch,
                    position: start_pos + updates.len() as u64,
                };
                updates.push(child_name, node.tree_version, commitment);
                std::mem::drop(node);
            }
        }
        tree_with_info.children_marks.clear();
        tree_with_info.mark_in_parent = false;
        let tree = &mut tree_with_info.tree;

        let dirty = tree.dirty();
        // let commitment = tree.flush();
        let commitment = if name.0.len() > 0 {
            tree.flush()
        } else {
            tree.commitment().clone()
        };

        return (dirty, commitment);
    }

    pub fn commit(
        &mut self,
        epoch: u64,
        start_pos: u64,
    ) -> (
        Commitment,
        impl IntoIterator<Item = (TreeName, u64, G1Aff<Pairing>)>,
    ) {
        let mut updates = SubTreeRootRecorder::with_capacity(1 << 20);
        let (_, commitment) = Self::commit_tree(
            &TreeName::root(),
            epoch,
            start_pos,
            &mut self.forest,
            &mut updates,
        );
        // for layers in self.forest.iter_mut().skip(1) {
        //     layers.clear();
        // }
        updates.into_affine_in_batch();

        return (commitment, updates);
    }
}

pub struct SubTreeRootRecorderIter(SubTreeRootRecorder);

impl Iterator for SubTreeRootRecorderIter {
    type Item = (TreeName, u64, G1Aff<Pairing>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((tree_name, version)) = self.0.tree_version.pop_front() {
            let commitment: G1Aff<Pairing> = self.0.commitments.pop_front().unwrap().into();
            Some((tree_name, version, commitment))
        } else {
            None
        }
    }
}

pub struct SubTreeRootRecorder {
    tree_version: VecDeque<(TreeName, u64)>,
    commitments: VecDeque<G1<Pairing>>,
}

impl SubTreeRootRecorder {
    fn with_capacity(n: usize) -> Self {
        Self {
            tree_version: VecDeque::with_capacity(n),
            commitments: VecDeque::with_capacity(n),
        }
    }

    fn push(&mut self, name: TreeName, version: u64, commitment: G1<Pairing>) {
        self.commitments.push_back(commitment);
        self.tree_version.push_back((name, version));
    }

    fn into_affine_in_batch(&mut self) {
        let (slice1, slice2) = self.commitments.as_mut_slices();
        assert_eq!(slice2.len(), 0);
        ProjectiveCurve::batch_normalization(slice1);
    }

    fn len(&self) -> usize {
        self.tree_version.len()
    }
}

impl IntoIterator for SubTreeRootRecorder {
    type Item = (TreeName, u64, G1Aff<Pairing>);
    type IntoIter = SubTreeRootRecorderIter;

    fn into_iter(self) -> Self::IntoIter {
        SubTreeRootRecorderIter(self)
    }
}

#[derive(Clone)]
struct TreeProducer {
    pub db: DBColumn,
    pub pp: Arc<AMTParams<Pairing>>,
    pub only_root: bool,
}

impl TreeProducer {
    fn new_tree(&self, name: &TreeName) -> Tree {
        Tree::new(
            name.clone(),
            self.db.clone(),
            self.pp.clone(),
            self.only_root,
        )
    }
}

#[derive(Default, Copy, Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct VerInfo {
    pub version: u64,
    pub level: u8,
    pub slot_index: u8,
}

impl ToBytes for VerInfo {
    fn write<W: Write>(&self, mut writer: W) -> ::std::io::Result<()> {
        self.version.write(&mut writer)?;
        self.level.write(&mut writer)?;
        self.slot_index.write(writer)?;
        Ok(())
    }
}

impl_storage_from_canonical!(VerInfo);

#[derive(Clone)]
struct TreeWithInfo {
    tree: Tree,
    mark_in_parent: bool,
    children_marks: HashSet<NodeIndex>,
}

fn fr_int_pow_2(power: u32) -> FrInt<Pairing> {
    let mut fr_int = FrInt::<Pairing>::from(1);
    fr_int.muln(power);
    fr_int
}
