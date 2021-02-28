use super::{Commitment, Key, Node, Tree, TreeName, IDX_MASK, MAX_VERSION_NUMBER};
use crate::crypto::export::{ToBytes, Write};
use crate::{
    crypto::{paring_provider::Pairing, AMTParams},
    storage::KvdbRocksdb,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone)]
pub struct TreeManager {
    db: KvdbRocksdb,
    pp: Arc<AMTParams<Pairing>>,
    forest: Vec<BTreeMap<u128, Tree>>,
}

impl TreeManager {
    fn new(db: KvdbRocksdb, pp: Arc<AMTParams<Pairing>>) -> Self {
        let mut forest = Vec::with_capacity(8);
        forest.push(BTreeMap::new());
        Self { db, forest, pp }
    }

    pub fn get(&self, name: TreeName) -> Option<&Tree> {
        let TreeName(level, index) = name;
        self.forest.get(level).and_then(|tree| tree.get(&index))
    }

    pub fn get_mut_or_load(&mut self, name: TreeName) -> &mut Tree {
        let TreeName(level, index) = name;
        if self.forest.len() < level + 1 {
            self.forest.resize(level + 1, BTreeMap::new())
        }
        self.forest[level]
            .entry(index)
            .or_insert(Tree::new(name, self.db.clone(), self.pp.clone()))
    }

    fn max_level(&self) -> usize {
        self.forest.len() - 1
    }

    fn get_neiboring_levels(
        &mut self,
        level: usize,
    ) -> (&mut BTreeMap<u128, Tree>, &mut BTreeMap<u128, Tree>) {
        let (parent_level, level) = self.forest[(level - 1)..=level].split_first_mut().unwrap();
        (parent_level, &mut level[0])
    }
}

pub struct VerForest {
    pub(crate) tree_manager: TreeManager,
    pub(crate) pp: Arc<AMTParams<Pairing>>,
}

#[derive(Default, Copy, Clone, Debug)]
pub struct VerInfo {
    pub version: u64,
    pub level: usize, //When serialized, `level` and `slot_index` are regarded as u8.
    pub slot_index: usize,
}

impl ToBytes for VerInfo {
    fn write<W: Write>(&self, mut writer: W) -> ::std::io::Result<()> {
        self.version.write(&mut writer)?;
        (self.level as u8).write(&mut writer)?;
        (self.slot_index as u8).write(writer)?;
        Ok(())
    }
}

impl VerForest {
    pub fn new(db: KvdbRocksdb, pp: Arc<AMTParams<Pairing>>) -> Self {
        Self {
            tree_manager: TreeManager::new(db, pp.clone()),
            pp,
        }
    }

    pub fn get_key(&mut self, key: &Key) -> VerInfo {
        let mut level = 0;
        let mut visit_amt = self.tree_manager.get_mut_or_load(TreeName::root());
        loop {
            let node_index = key.index_at_level(level) as usize;
            let node: &Node = visit_amt.get(node_index);
            for (slot_index, (node_key, ver)) in node.key_versions.iter().enumerate() {
                if *key == *node_key {
                    return VerInfo {
                        version: *ver as u64,
                        level,
                        slot_index,
                    };
                }
            }

            let num_alloc_slots = node.key_versions.len();

            level += 1;
            let tree_name = TreeName::from_key_level(key, level);

            // In case the subtree does not exist
            if node.tree_version == 0
                && self
                    .tree_manager
                    .get(tree_name)
                    .map_or(true, |tree| !tree.dirty())
            {
                // returns the empty slot that allocate for this key.
                return if num_alloc_slots < 5 {
                    VerInfo {
                        version: 0,
                        level: level - 1,
                        slot_index: num_alloc_slots,
                    }
                } else {
                    VerInfo {
                        version: 0,
                        level,
                        slot_index: 0,
                    }
                };
            }

            visit_amt = self.tree_manager.get_mut_or_load(tree_name);
        }
    }

    pub fn inc_key(&mut self, key: &Key) -> VerInfo {
        let mut level = 0;
        let mut visit_amt = self.tree_manager.get_mut_or_load(TreeName::root());
        loop {
            let node_index = key.index_at_level(level) as usize;

            debug!(
                "Access key {:?} at level {}, tree_index {}, node_index {}",
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
                        slot_index,
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
                    slot_index,
                };
            }

            // Drop `node_guard` even if nothing changes.
            std::mem::drop(node_guard);

            // Access the next level.
            level += 1;
            visit_amt = self
                .tree_manager
                .get_mut_or_load(TreeName::from_key_level(key, level));
        }
    }

    pub fn commit(&mut self) -> Vec<(TreeName, Commitment, u64)> {
        let db = self.tree_manager.db.clone();
        let pp = self.pp.clone();
        let max_level = self.tree_manager.max_level();

        let mut update = Vec::new();

        for level in (1..=max_level).rev() {
            let (parent_level_trees, level_trees) = self.tree_manager.get_neiboring_levels(level);
            for (&index, tree) in level_trees.iter_mut().filter(|(_index, tree)| tree.dirty()) {
                tree.flush();

                let TreeName(parent_level, parent_index) = TreeName(level, index).parent().unwrap();
                let default_tree =
                    || Tree::new(TreeName(parent_level, parent_index), db.clone(), pp.clone());
                let mut parent_node_guard = parent_level_trees
                    .entry(parent_index)
                    .or_insert_with(default_tree)
                    .write((index & IDX_MASK as u128) as usize);

                let ver = &mut parent_node_guard.tree_version;
                *ver += 1;
                assert!(*ver <= MAX_VERSION_NUMBER);

                update.push((TreeName(level, index), tree.commitment().clone(), *ver));
            }
        }

        self.tree_manager.get_mut_or_load(TreeName::root());

        update.sort_unstable_by_key(|(name, _, _)| *name);

        return update;
    }

    fn commitment(&mut self) -> Commitment {
        self.tree_manager
            .get_mut_or_load(TreeName::root())
            .commitment()
            .clone()
    }
}
