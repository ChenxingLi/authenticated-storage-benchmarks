use super::{Commitment, Key, Node, Tree, TreeName, MAX_VERSION_NUMBER};
use crate::amt::{paring_provider::Pairing, prove_params::AMTParams, DEPTHS, IDX_MASK};
use crate::storage::KvdbRocksdb;
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

    fn get(&self, name: TreeName) -> Option<&Tree> {
        let TreeName(level, index) = name;
        self.forest.get(level).and_then(|tree| tree.get(&index))
    }

    fn get_mut_or_load(&mut self, name: TreeName) -> &mut Tree {
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

    fn get_mut_levels(
        &mut self,
        level: usize,
    ) -> (&mut BTreeMap<u128, Tree>, &mut BTreeMap<u128, Tree>) {
        let (parent_level, level) = self.forest[(level - 1)..=level].split_first_mut().unwrap();
        (parent_level, &mut level[0])
    }
}

pub struct VerForest {
    tree_manager: TreeManager,
    pp: Arc<AMTParams<Pairing>>,
}

impl VerForest {
    fn new(db: KvdbRocksdb, pp: Arc<AMTParams<Pairing>>) -> Self {
        Self {
            tree_manager: TreeManager::new(db, pp.clone()),
            pp,
        }
    }

    fn get(&mut self, key: &Key) -> u64 {
        let mut level = 0;
        let mut visit_amt = self.tree_manager.get_mut_or_load(TreeName::root());
        loop {
            let node_index = key.index_at_level(level) as usize;
            let node: &Node = visit_amt.get(node_index);
            for (node_key, ver) in &node.key_versions {
                if *key == *node_key {
                    return *ver as u64;
                }
            }

            level += 1;
            let tree_name = TreeName::from_key_level(key, level);

            if node.tree_version == 0
                && self
                    .tree_manager
                    .get(tree_name)
                    .map_or(true, |tree| !tree.dirty())
            {
                return 0;
            }

            visit_amt = self.tree_manager.get_mut_or_load(tree_name);
        }
    }

    fn inc(&mut self, key: &Key) -> u64 {
        let mut level = 0;
        let mut visit_amt = self.tree_manager.get_mut_or_load(TreeName::root());
        loop {
            let node_index = key.index_at_level(level) as usize;
            let mut tree_guard = visit_amt.write(node_index);
            for (node_key, ver) in &mut tree_guard.key_versions {
                if *key == *node_key {
                    *ver += 1;
                    assert!(*ver < MAX_VERSION_NUMBER);
                    return *ver;
                }
            }
            if tree_guard.key_versions.len() < 5 {
                tree_guard.key_versions.push((key.clone(), 1));
                return 1;
            }

            std::mem::drop(tree_guard);

            level += 1;
            visit_amt = self
                .tree_manager
                .get_mut_or_load(TreeName::from_key_level(key, level));
        }
    }

    fn commit(&mut self) -> BTreeMap<TreeName, (Commitment, u64)> {
        let db = self.tree_manager.db.clone();
        let pp = self.pp.clone();
        let max_level = self.tree_manager.max_level();

        let mut update = BTreeMap::<TreeName, (Commitment, u64)>::new();

        for level in (1..=max_level).rev() {
            let (parent_level_trees, level_trees) = self.tree_manager.get_mut_levels(level);
            for (&index, tree) in level_trees.iter_mut().filter(|(_index, tree)| tree.dirty()) {
                tree.flush();

                let parent_index = index >> DEPTHS;
                let default_tree =
                    || Tree::new(TreeName(level - 1, parent_index), db.clone(), pp.clone());
                let mut parent_node_guard = parent_level_trees
                    .entry(parent_index)
                    .or_insert_with(default_tree)
                    .write((index & IDX_MASK as u128) as usize);

                let version = &mut parent_node_guard.tree_version;
                *version += 1;
                assert!(*version < MAX_VERSION_NUMBER);

                let commitment = tree.commitment().clone();

                update.insert(TreeName(level, index), (commitment, *version));
            }
        }

        return update;
    }
}
