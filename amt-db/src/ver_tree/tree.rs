use super::{Commitment, Key, Node, Tree, TreeName, MAX_VERSION_NUMBER};
use crate::crypto::export::G1;
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
}

impl VersionTree {
    pub fn new(db: KvdbRocksdb, pp: Arc<AMTParams<Pairing>>, only_root: bool) -> Self {
        let mut forest = Vec::with_capacity(8);
        forest.push(BTreeMap::new());
        Self {
            forest,
            producer: TreeProducer { db, pp, only_root },
            root: Default::default(),
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
                last_tree.set_commitment(&tree_with_info.tree.get(sub_index as usize).commitment);
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

    pub fn get_key_info(&mut self, key: &Key) -> VerInfo {
        let mut level = 0;
        let mut visit_amt = self.get_tree_mut(&TreeName::root());
        loop {
            let node_index = key.index_at_level(level) as usize;
            let node: &Node = visit_amt.get(node_index);
            for (slot_index, (node_key, ver)) in node.key_versions.iter().enumerate() {
                if *key == *node_key {
                    return VerInfo {
                        version: *ver as u64,
                        level,
                        slot_index: slot_index as u8,
                    };
                }
            }

            let num_alloc_slots = node.key_versions.len();

            level += 1;
            let tree_name = TreeName::from_key_level(key, level);

            // In case the subtree does not exist
            if node.tree_version == 0
                && self.get_tree(&tree_name).map_or(true, |tree| !tree.dirty())
            {
                // returns the empty slot that allocate for this key.
                return if num_alloc_slots < 5 {
                    VerInfo {
                        version: 0,
                        level: level - 1,
                        slot_index: num_alloc_slots as u8,
                    }
                } else {
                    VerInfo {
                        version: 0,
                        level,
                        slot_index: 0,
                    }
                };
            }

            visit_amt = self.get_tree_mut(&tree_name);
        }
    }

    pub fn inc_key_ver(&mut self, key: &Key) -> VerInfo {
        let mut level = 0;
        let mut visit_amt = self.get_tree_mut(&TreeName::root());
        loop {
            let node_index = key.index_at_level(level) as usize;

            debug!(
                "Access key {:?} at level {}, tree_index {:?}, node_index {}",
                key.0,
                level,
                key.tree_at_level(level),
                node_index
            );

            let mut node_guard = visit_amt.write(node_index);
            for (slot_index, (ref mut node_key, ver)) in
                &mut node_guard.key_versions.iter_mut().enumerate()
            {
                if *key == *node_key {
                    *ver += 1;
                    assert!(*ver <= MAX_VERSION_NUMBER);
                    // std::mem::drop(node_guard);
                    return VerInfo {
                        version: *ver as u64,
                        level,
                        slot_index: slot_index as u8,
                    };
                }
            }

            // In case this level is not fulfilled, put the
            if node_guard.key_versions.len() < 5 {
                let slot_index = node_guard.key_versions.len();
                node_guard.key_versions.push((key.clone(), 1));

                // std::mem::drop(node_guard);
                return VerInfo {
                    version: 1,
                    level,
                    slot_index: slot_index as u8,
                };
            }

            // Drop `node_guard` even if nothing changes.
            std::mem::drop(node_guard);

            // Access the next level.
            level += 1;
            visit_amt = self.get_tree_mut(&TreeName::from_key_level(key, level));
        }
    }

    fn commit_tree(
        name: &TreeName,
        layers: &mut [TreesLayer],
        updates: &mut Vec<(TreeName, Commitment, u64)>,
    ) -> (bool, Commitment) {
        let (this_layer, rest_layers) = layers.split_first_mut().unwrap();

        let tree_with_info = this_layer.get_mut(&name.0).unwrap();

        for &index in tree_with_info.children_marks.iter() {
            let child_name = name.child(index);
            let (dirty, commitment) = Self::commit_tree(&child_name, rest_layers, updates);
            if dirty {
                let node = &mut tree_with_info.tree.write(index as usize);
                node.tree_version += 1;
                node.commitment = commitment;
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

    pub fn commit(&mut self) -> (Commitment, Vec<(TreeName, Commitment, u64)>) {
        let mut updates = Vec::with_capacity(1024);
        let (_, commitment) = Self::commit_tree(&TreeName::root(), &mut self.forest, &mut updates);
        updates.sort_unstable_by_key(|(name, _, _)| name.clone());

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
