use super::{Commitment, Key, Node, Tree, TreeName, MAX_VERSION_NUMBER};
use crate::crypto::export::{BigInteger, FrInt, G1};
use crate::ver_tree::node::EpochPosition;
use crate::{
    crypto::{
        export::{CanonicalDeserialize, CanonicalSerialize, Pairing, SerializationError, ToBytes},
        AMTParams,
    },
    impl_storage_from_canonical,
    storage::{KvdbRocksdb, StorageDecodable, StorageEncodable},
};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::{collections::BTreeMap, sync::Arc};

type NodeIndex = u32;
type TreesLayer = BTreeMap<Vec<NodeIndex>, TreeWithInfo>;

/// The `VersionTree`
#[derive(Clone)]
pub struct VersionTree {
    root: G1<Pairing>,
    producer: TreeProducer,
    forest: Vec<TreesLayer>,

    fast_mode: bool,
}

impl VersionTree {
    pub fn new(db: KvdbRocksdb, pp: Arc<AMTParams<Pairing>>, only_root: bool) -> Self {
        let mut forest = Vec::with_capacity(8);
        forest.push(BTreeMap::new());
        Self {
            forest,
            producer: TreeProducer { db, pp, only_root },
            root: Default::default(),
            fast_mode: only_root,
        }
    }

    fn get_tree(&self, name: &TreeName) -> Option<&Tree> {
        self.forest
            .get(name.0.len())
            .and_then(|tree| tree.get(&name.0))
            .map(|tree_with_info| &tree_with_info.tree)
    }

    pub(crate) fn get_tree_mut(&mut self, name: &TreeName) -> &mut Tree {
        let (ancestor_layers, tree_layer) = {
            let level = name.0.len();
            if self.forest.len() < level + 1 {
                self.forest.resize(level + 1, BTreeMap::new())
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

    // pub fn get_key_info(&mut self, key: &Key) -> VerInfo {
    //     let mut level = 0;
    //     let mut visit_amt = self.get_tree_mut(&TreeName::root());
    //     loop {
    //         let node_index = key.index_at_level(level) as usize;
    //         let node: &Node = visit_amt.get(node_index);
    //         for (slot_index, (node_key, ver)) in node.key_versions.iter().enumerate() {
    //             if *key == *node_key {
    //                 return VerInfo {
    //                     version: *ver as u64,
    //                     level,
    //                     slot_index: slot_index as u8,
    //                 };
    //             }
    //         }
    //
    //         let num_alloc_slots = node.key_versions.len();
    //
    //         level += 1;
    //         let tree_name = TreeName::from_key_level(key, level);
    //
    //         // In case the subtree does not exist
    //         if node.tree_version == 0
    //             && self.get_tree(&tree_name).map_or(true, |tree| !tree.dirty())
    //         {
    //             // returns the empty slot that allocate for this key.
    //             return if num_alloc_slots < 5 {
    //                 VerInfo {
    //                     version: 0,
    //                     level: level - 1,
    //                     slot_index: num_alloc_slots as u8,
    //                 }
    //             } else {
    //                 VerInfo {
    //                     version: 0,
    //                     level,
    //                     slot_index: 0,
    //                 }
    //             };
    //         }
    //
    //         visit_amt = self.get_tree_mut(&tree_name);
    //     }
    // }

    pub fn inc_key_ver(&mut self, key: &Key, version: Option<VerInfo>) -> VerInfo {
        let VerInfo {
            version,
            level,
            slot_index,
        } = match version {
            None => self.allocate_vacant_slot(key),
            Some(ver_info) => ver_info,
        };

        let mut visit_amt = self.get_tree_mut(&key.tree_at_level(level));
        let node = key.index_at_level(level);
        if visit_amt.only_root() {
            visit_amt.update(node, fr_int_pow_2((slot_index as u32 + 1) * 5));
        } else {
            let mut node_guard = visit_amt.write_versions(node);
            node_guard.key_versions[slot_index as usize] += 1;
        }
        return VerInfo {
            version: version + 1,
            level,
            slot_index,
        };
    }

    pub fn allocate_vacant_slot(&mut self, key: &Key) -> VerInfo {
        for level in 0..32 {
            let mut visit_amt = self.get_tree_mut(&key.tree_at_level(level));
            let mut node_guard = visit_amt.write_versions(key.index_at_level(level));

            if node_guard.key_versions.len() < 5 {
                let slot_index = node_guard.key_versions.len();
                node_guard.key_versions.push(0);

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
        updates: &mut Vec<(TreeName, Commitment, u64)>,
    ) -> (bool, Commitment) {
        let (this_layer, rest_layers) = layers.split_first_mut().unwrap();

        let tree_with_info = this_layer.get_mut(&name.0).unwrap();

        for &index in tree_with_info.children_marks.iter() {
            let child_name = name.child(index);
            let (dirty, commitment) =
                Self::commit_tree(&child_name, epoch, start_pos, rest_layers, updates);
            if dirty {
                *tree_with_info.tree.subtree_root_mut(index as usize) = commitment;
                let node = &mut tree_with_info.tree.write_versions(index as usize);
                node.tree_version += 1;
                node.tree_position = EpochPosition {
                    epoch,
                    position: start_pos + updates.len() as u64,
                };
                updates.push((child_name, commitment, node.tree_version));
            }
        }
        tree_with_info.children_marks.clear();
        tree_with_info.mark_in_parent = false;
        let tree = &mut tree_with_info.tree;

        let dirty = tree.dirty();
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
    ) -> (Commitment, Vec<(TreeName, Commitment, u64)>) {
        let mut updates = Vec::with_capacity(1024);
        let (_, commitment) = Self::commit_tree(
            &TreeName::root(),
            epoch,
            start_pos,
            &mut self.forest,
            &mut updates,
        );
        // updates.sort_unstable_by_key(|(name, _, _)| name.clone());

        return (commitment, updates);
    }
}

#[derive(Clone)]
struct TreeProducer {
    pub db: KvdbRocksdb,
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
    children_marks: BTreeSet<NodeIndex>,
}

fn fr_int_pow_2(power: u32) -> FrInt<Pairing> {
    let mut fr_int = FrInt::<Pairing>::from(1);
    fr_int.muln(power);
    fr_int
}
