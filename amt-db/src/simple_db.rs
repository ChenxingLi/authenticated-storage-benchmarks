use crate::amt::tree::AMTProof;
use crate::amt::{AMTData, AMTree};
use crate::crypto::{
    paring_provider::{Fr, FrInt, G1},
    AMTParams, Pairing,
};
use crate::merkle::{MerkleProof, StaticMerkleTree};
use crate::storage::{
    KeyValueDbTrait, KeyValueDbTraitRead, KvdbRocksdb, Result, StorageDecodable, StorageEncodable,
    StoreByBytes, StoreTupleByBytes, SystemDB,
};
use crate::ver_tree::{AMTConfig, Commitment, Key, Node, TreeName, VerForest, VerInfo};
use algebra::bls12_381::G1Projective;
use cfx_types::H256;
use keccak_hash::keccak;
use std::collections::VecDeque;
use std::sync::Arc;

const COL_VER_TREE: u32 = 0;
const COL_KEY_POS: u32 = COL_VER_TREE + 1;
const COL_TREE_POS: u32 = COL_KEY_POS + 1;
const COL_POS_VALUE: u32 = COL_TREE_POS + 1;
const COL_POS_VALUE_MERKLE: u32 = COL_POS_VALUE + 1;
const NUM_COLS: u32 = COL_POS_VALUE_MERKLE + 1;

const WRITE_IN_BATCH: bool = false;

struct SimpleDb {
    version_tree: VerForest,
    db_key_pos: KvdbRocksdb,
    db_tree_pos: KvdbRocksdb,
    db_pos_value: KvdbRocksdb,
    db_pos_value_merkle: KvdbRocksdb,

    uncommitted_key_values: Vec<(Key, Box<[u8]>)>,
    dirty_guard: bool,
}

/// I can not implement `impl StoreByBytes for Commitment {}` here. So I implement for
/// `G1Projective` instead.
/// See https://github.com/arkworks-rs/algebra/issues/185 for more details.  
impl StoreByBytes for G1Projective {}
impl StoreTupleByBytes for (u64, u64) {}
impl StoreTupleByBytes for (Vec<u8>, VerInfo, Vec<u8>) {}
impl StoreTupleByBytes for (TreeName, u64, Commitment) {}

#[derive(Default)]
struct LevelProof {
    merkle_epoch: u64,
    merkle_proof: MerkleProof,
    amt_proof: AMTProof<G1<Pairing>>,
    commitment: G1<Pairing>,
    node_fr_int: FrInt<Pairing>,
    node_version: u64,
}

#[derive(Default)]
struct AssociateProof {
    value: Option<Box<[u8]>>,
    ver_info: VerInfo,
}

type Proof = (AssociateProof, VecDeque<LevelProof>);

impl SimpleDb {
    fn new(database: Arc<SystemDB>, pp: Arc<AMTParams<Pairing>>) -> Self {
        let db_ver_tree = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_VER_TREE,
        };
        let version_tree = VerForest::new(db_ver_tree, pp);
        let db_key_pos = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_KEY_POS,
        };
        let db_tree_current = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_TREE_POS,
        };
        let db_pos_value = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_POS_VALUE,
        };
        let db_pos_value_merkle = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_POS_VALUE_MERKLE,
        };
        Self {
            version_tree,
            db_key_pos,
            db_pos_value,
            db_pos_value_merkle,
            db_tree_pos: db_tree_current,
            uncommitted_key_values: Vec::new(),
            dirty_guard: false,
        }
    }

    fn get(&self, key: &Key) -> Result<Option<Box<[u8]>>> {
        assert!(
            !self.dirty_guard,
            "Can not read db if set operations have not been committed."
        );

        let maybe_pos = self.db_key_pos.get(key.as_ref())?;
        if let Some(pos) = maybe_pos {
            return Ok(Some(
                self.db_pos_value.get(&pos)?.expect("Should find a key"),
            ));
        }
        Ok(None)
    }

    fn set(&mut self, key: &Key, value: Box<[u8]>) {
        self.dirty_guard = true;
        self.uncommitted_key_values.push((key.clone(), value))
    }

    fn commit(&mut self, epoch: u64) -> Result<()> {
        let kv_num = self.uncommitted_key_values.len();
        let mut hashes = Vec::with_capacity(kv_num);

        for (position, (key, value)) in self.uncommitted_key_values.drain(..).enumerate() {
            let position = position as u64;
            let ver_info = self.version_tree.inc_key(&key);

            self.db_key_pos
                .put(key.as_ref(), &(epoch, position).storage_encode())?;

            self.db_pos_value
                .put(&(epoch, position).storage_encode(), &value)?;

            let key_ver_value_hash = keccak(&(key.0, ver_info, value.to_vec()).storage_encode());
            hashes.push(key_ver_value_hash);
        }

        for (position, (tree, commitment, version)) in
            self.version_tree.commit().drain(..).enumerate()
        {
            let position = (kv_num + position) as u64;

            self.db_tree_pos
                .put(&tree.storage_encode(), &(epoch, position).storage_encode())?;

            self.db_pos_value.put(
                &(epoch, position).storage_encode(),
                &commitment.storage_encode(),
            )?;

            let name_ver_value_hash = keccak(&(tree, version, commitment).storage_encode());
            hashes.push(name_ver_value_hash);
        }

        StaticMerkleTree::dump(self.db_pos_value_merkle.clone(), epoch, hashes);

        self.dirty_guard = false;

        Ok(())
    }

    #[allow(unused_variables, unused_mut)]
    fn prove(&mut self, key: &Key) -> Result<Proof> {
        let ver_info = self.version_tree.get_key(&key);
        let maybe_pos = self.db_key_pos.get(key.as_ref())?;
        let maybe_value = if let Some(pos) = &maybe_pos {
            Some(self.db_pos_value.get(pos)?.expect("Should find a key"))
        } else {
            None
        };

        let assoc_proof = AssociateProof {
            value: maybe_value,
            ver_info,
        };

        // Key value Merkle proof
        let (merkle_epoch, merkle_proof) = if let Some(pos) = &maybe_pos {
            self.prove_merkle(pos)?
        } else {
            (0, MerkleProof::default())
        };

        // AMT Proof
        let tree_name = TreeName::from_key_level(key, ver_info.level);
        let index = key.index_at_level(ver_info.level) as usize;
        let (commitment, node, amt_proof) = self.prove_amt_node(tree_name, index);

        let mut level_proofs = VecDeque::with_capacity(ver_info.level + 1);

        level_proofs.push_back(LevelProof {
            merkle_epoch,
            merkle_proof,
            amt_proof,
            commitment,
            node_fr_int: node.as_fr_int(),
            node_version: node.key_versions[ver_info.slot_index].1,
        });

        for level in (0..ver_info.level).rev() {
            let child_tree_name = TreeName::from_key_level(key, level + 1);
            let pos = self
                .db_tree_pos
                .get(&child_tree_name.storage_encode())?
                .expect("the child_tree node should exists");

            let (merkle_epoch, merkle_proof) = self.prove_merkle(&pos)?;

            let tree_name = TreeName::from_key_level(key, level);
            let index = key.index_at_level(level) as usize;
            let (commitment, node, amt_proof) = self.prove_amt_node(tree_name, index);

            level_proofs.push_back(LevelProof {
                merkle_epoch,
                merkle_proof,
                amt_proof,
                commitment,
                node_fr_int: node.as_fr_int(),
                node_version: node.tree_version,
            });
        }

        Ok((assoc_proof, level_proofs))
    }

    fn verify<F: Fn(u64) -> H256>(
        key: &Key,
        proof: &Proof,
        epoch_root: F,
        pp: &AMTParams<Pairing>,
    ) -> std::result::Result<(), String> {
        let (assoc_proof, level_proofs) = proof;

        let ver_info = assoc_proof.ver_info;

        // Check the AMT proof
        for (level, level_proof) in level_proofs.iter().enumerate() {
            let amt_index = key.index_at_level(level) as usize;
            let amt_proof_verified = AMTree::<AMTConfig>::verify(
                amt_index,
                Fr::<Pairing>::from(level_proof.node_fr_int),
                &level_proof.commitment,
                level_proof.amt_proof.clone(),
                pp,
            );
            if !amt_proof_verified {
                return Err(format!("Incorrect AMT proof at level {}", level));
            }
        }

        // Check Merkle proof in the top level.
        if let Some(value) = &assoc_proof.value {
            let key_ver_value_hash =
                keccak(&(key.0.clone(), ver_info, value.to_vec()).storage_encode());

            let epoch = level_proofs[0].merkle_epoch;

            let merkle_proof = &level_proofs[0].merkle_proof;

            let merkle_proof_verified =
                StaticMerkleTree::verify(&epoch_root(epoch), &key_ver_value_hash, merkle_proof);

            if !merkle_proof_verified {
                return Err("Incorrect Merkle proof at level 0".to_string());
            }
        }

        // Check Merkle proof in the rest levels.
        for level in 1..level_proofs.len() {
            let version = level_proofs[level - 1].node_version;
            let level_proof = &level_proofs[level];
            let tree_name = TreeName::from_key_level(key, level);
            let commitment = level_proof.commitment;

            let key_ver_value_hash = keccak(&(tree_name, version, commitment).storage_encode());
            let epoch = level_proof.merkle_epoch;
            let merkle_proof = &level_proof.merkle_proof;

            let merkle_proof_verified =
                StaticMerkleTree::verify(&epoch_root(epoch), &key_ver_value_hash, merkle_proof);

            if !merkle_proof_verified {
                return Err(format!("Incorrect Merkle proof at level {}", level));
            }
        }

        // Check version consistency in the top level.
        let version_verified =
            Node::versions_from_fr_int(&level_proofs[0].node_fr_int, ver_info.slot_index + 1)
                == level_proofs[0].node_version;
        if !version_verified {
            return Err(format!("Inconsistent version value at level 0"));
        }

        // Check version consistency in the rest levels.
        for (level, level_proof) in level_proofs.iter().enumerate().skip(1) {
            let version_verified =
                Node::versions_from_fr_int(&level_proof.node_fr_int, 0) == level_proof.node_version;

            if !version_verified {
                return Err(format!("Inconsistent version value at level {}", level));
            }
        }

        Ok(())
    }

    fn prove_amt_node(
        &mut self,
        name: TreeName,
        index: usize,
    ) -> (G1<Pairing>, Node, AMTProof<G1<Pairing>>) {
        let tree = self.version_tree.tree_manager.get_mut_or_load(name);

        let commitment = tree.commitment().clone();
        let value = tree.get(index).clone();
        let proof = tree.prove(index);

        return (commitment, value, proof);
    }

    fn prove_merkle(&mut self, pos: &[u8]) -> Result<(u64, MerkleProof)> {
        let (epoch, position) = <(u64, u64)>::storage_decode(pos)?;
        let merkle_proof =
            StaticMerkleTree::new(self.db_pos_value_merkle.clone(), epoch).prove(position);

        Ok((epoch, merkle_proof))
    }
}

#[cfg(test)]
fn new_test_simple_db(dir: &str) -> SimpleDb {
    use crate::crypto::{TypeDepths, TypeUInt, PP};
    use crate::storage::open_database;
    const DEPTHS: usize = TypeDepths::USIZE;

    let db = open_database(dir, NUM_COLS);
    let pp = PP::<Pairing>::from_file_or_new("./pp", DEPTHS);
    let amt_param = Arc::new(AMTParams::<Pairing>::from_pp(pp, DEPTHS));

    SimpleDb::new(db, amt_param)
}

#[test]
fn test_simple_db() {
    // use crate::enable_log::enable_debug_log;
    // enable_debug_log();

    let mut db = new_test_simple_db("./__test_simple_db");

    for i in 1..=255 {
        db.set(&Key(vec![1, 2, i, 0]), vec![1, 2, i].into());
        db.commit(i as u64).unwrap();
    }

    for i in 1..=20 {
        assert_eq!(
            vec![1, 2, i],
            db.get(&Key(vec![1, 2, i, 0])).unwrap().unwrap().into_vec()
        );
    }

    std::fs::remove_dir_all("./__test_simple_db").unwrap();
}
