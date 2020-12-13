use super::{Key, Node, Tree, TreeName};
use crate::amt::{paring_provider::Pairing, prove_params::AMTParams};
use crate::storage::KvdbRocksdb;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub struct TreeManager {
    db: KvdbRocksdb,
    pp: Arc<AMTParams<Pairing>>,
    forest: HashMap<TreeName, Tree>,
}

impl TreeManager {
    fn new(db: KvdbRocksdb, pp: Arc<AMTParams<Pairing>>) -> Self {
        Self {
            db,
            forest: HashMap::new(),
            pp,
        }
    }

    fn get_mut(&mut self, name: TreeName) -> &mut Tree {
        self.forest
            .entry(name)
            .or_insert(Tree::new(name, self.db.clone(), self.pp.clone()))
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
        let mut visit_amt = self.tree_manager.get_mut(TreeName::Root);
        loop {
            let node_index = key.index_at_level(level) as usize;
            let node: &Node = visit_amt.get(node_index);
            for (node_key, ver) in &node.key_versions {
                if *key == *node_key {
                    return *ver as u64;
                }
            }
            if node.tree_version == 0 {
                return 0;
            }
            level += 1;
            visit_amt = self
                .tree_manager
                .get_mut(TreeName::Subtree(level, key.tree_at_level(level)));
        }
    }

    // fn inc(&mut self, key: &Key) {
    //     let mut level = 0;
    //     let mut visit_amt = self.tree_manager.get_mut(TreeName::Root);
    //     loop {
    //         let node_index = key.index_at_level(level) as usize;
    //         match visit_amt.get(node_index) {
    //             Node::Empty => {
    //                 let versions = vec![1u32];
    //                 let keys = vec![key.clone()];
    //                 let fr_int = make_fr_int(&versions);
    //
    //                 let update = |x: &mut Node| *x = Node::Squeeze(versions, keys, fr_int);
    //                 visit_amt.update(node_index, update, &self.pp);
    //                 return;
    //             }
    //             Node::Squeeze(versions, keys, _) => {
    //                 let entry = keys
    //                     .iter()
    //                     .zip(versions.iter())
    //                     .enumerate()
    //                     .find(|&(_, (k, _ver))| *k == key);
    //
    //                 // TODO: replace later.
    //                 if let Some((index, _key, ver)) = entry {
    //                     if ver < i32::MAX - 1 {
    //                         let update = |x: &mut Node| {
    //                             if let Node::Squeeze(vers, _, fr_int) = x {
    //                                 vers[index] += 1;
    //                                 *fr_int += FrInt::from(1).muln(31 * index);
    //                             } else {
    //                                 unreachable!()
    //                             }
    //                         };
    //                     } else {
    //                         // TODO: expand to subtree
    //                         unimplemented!()
    //                     }
    //                 } else {
    //                     if keys.len() < 7 {
    //                         let update = |x: &mut Node| {
    //                             let vers =
    //                             if let Node::Squeeze(vers, _, fr_int) = x {
    //                                 vers[index] += 1;
    //                                 *fr_int += FrInt::from(1).muln(31 * index);
    //                             } else {
    //                                 unreachable!()
    //                             }
    //                         };
    //                     } else {
    //                     }
    //                 }
    //                 for (ver, node_key) in versions.iter().zip(keys.iter()) {
    //                     if *key == *node_key {
    //                         return *ver as u64;
    //                     }
    //                 }
    //                 return 0;
    //             }
    //             Node::Squeeze(versions, keys, _) if !keys.contains(key) => {
    //                 for (ver, node_key) in versions.iter().zip(keys.iter()) {
    //                     if *key == *node_key {
    //                         return *ver as u64;
    //                     }
    //                 }
    //                 return 0;
    //             }
    //             Node::NodeComm(version, node_key) => unimplemented!(),
    //             Node::TreeComm(_) => {
    //                 level += 1;
    //                 visit_amt = self
    //                     .tree_manager
    //                     .get_mut(TreeName::Subtree(level, key.tree_at_level(level)))
    //             }
    //         }
    //     }
    // }
}
