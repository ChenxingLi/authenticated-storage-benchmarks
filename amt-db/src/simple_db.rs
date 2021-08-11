#![allow(unused)]
use crate::amt::tree::AMTProof;
use crate::amt::{AMTData, AMTree};
use crate::crypto::{
    export::{Fr, FrInt, G1Projective, G1},
    AMTParams, Pairing, TypeUInt,
};
use crate::impl_storage_from_canonical;
use crate::merkle::{MerkleProof, StaticMerkleTree};
use crate::storage::{
    KeyValueDbTrait, KeyValueDbTraitRead, KvdbRocksdb, Result, StorageDecodable, StorageEncodable,
    SystemDB,
};
use crate::ver_tree::{AMTConfig, EpochPosition, Key, Node, TreeName, VerInfo, VersionTree};
use cfx_types::H256;
use keccak_hash::keccak;
use std::collections::VecDeque;
use std::sync::Arc;

const COL_VER_TREE: u32 = 0;
const COL_KEY_POS: u32 = COL_VER_TREE + 1;
const COL_TREE_POS: u32 = COL_KEY_POS + 1;
const COL_POS_VALUE: u32 = COL_TREE_POS + 1;
const COL_POS_VALUE_MERKLE: u32 = COL_POS_VALUE + 1;
const COL_KEY_NEW: u32 = COL_POS_VALUE_MERKLE + 1;
const COL_TREE_NEW: u32 = COL_KEY_NEW + 1;
const COL_MERKLE: u32 = COL_TREE_NEW + 1;
const NUM_COLS: u32 = COL_MERKLE + 1;

pub static INC_KEY_LEVEL_SUM: Global<u64> = Global::INIT;
pub static INC_KEY_COUNT: Global<u64> = Global::INIT;
pub static INC_TREE_COUNT: Global<u64> = Global::INIT;

pub struct SimpleDb {
    version_tree: VersionTree,
    db_key_pos: KvdbRocksdb,
    db_tree_pos: KvdbRocksdb,
    db_pos_value: KvdbRocksdb,
    db_pos_value_merkle: KvdbRocksdb,

    db_key_new: KvdbRocksdb,
    db_tree_new: KvdbRocksdb,
    db_merkle: KvdbRocksdb,

    uncommitted_key_values: Vec<(Key, Box<[u8]>)>,
    dirty_guard: bool,
    only_root: bool,
}

#[allow(unused)]
mod metadata {
    use super::EpochPosition;
    use super::{impl_storage_from_canonical, TreeName, VerInfo};
    use crate::crypto::export::G1Aff;
    use crate::crypto::{
        export::{
            CanonicalDeserialize, CanonicalSerialize, FromBytes, SerializationError, ToBytes, G1,
        },
        Pairing,
    };
    use crate::storage::{StorageDecodable, StorageEncodable};
    use std::io::{Read, Write};

    #[derive(Default, Clone, CanonicalDeserialize, CanonicalSerialize)]
    pub struct TreeValue {
        pub(crate) key: TreeName,
        pub(crate) version_number: u64,
        pub(crate) commitment: G1Aff<Pairing>,
    }
    impl_storage_from_canonical!(TreeValue);

    #[derive(Default, Clone, CanonicalDeserialize, CanonicalSerialize)]
    pub struct KeyValue {
        pub(crate) key: Vec<u8>,
        pub(crate) version: VerInfo,
        pub(crate) value: Vec<u8>,
    }
    impl_storage_from_canonical!(KeyValue);

    #[derive(Default, Clone, CanonicalDeserialize, CanonicalSerialize)]
    pub struct Value {
        pub(crate) value: Vec<u8>,
        pub(crate) version: VerInfo,
        pub(crate) position: EpochPosition,
    }
    impl_storage_from_canonical!(Value);
}

use crate::crypto::export::{G1Aff, ProjectiveCurve};
use global::Global;
use metadata::*;

#[derive(Default)]
pub struct LevelProof {
    merkle_epoch: u64,
    merkle_proof: MerkleProof,
    amt_proof: AMTProof<G1<Pairing>>,
    commitment: G1<Pairing>,
    node_fr_int: FrInt<Pairing>,
    node_version: u64,
}

#[derive(Default)]
pub struct AssociateProof {
    value: Option<Vec<u8>>,
    ver_info: VerInfo,
}

type Proof = (AssociateProof, VecDeque<LevelProof>);

impl SimpleDb {
    pub fn new(database: Arc<SystemDB>, pp: Arc<AMTParams<Pairing>>, only_root: bool) -> Self {
        let db_ver_tree = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_VER_TREE,
        };
        let version_tree = VersionTree::new(db_ver_tree, pp, only_root);
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
        let db_key_new = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_KEY_NEW,
        };
        let db_tree_new = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_TREE_NEW,
        };
        let db_merkle = KvdbRocksdb {
            kvdb: database.key_value().clone(),
            col: COL_MERKLE,
        };
        Self {
            version_tree,
            db_key_pos,
            db_pos_value,
            db_pos_value_merkle,
            db_tree_pos: db_tree_current,
            db_key_new,
            db_tree_new,
            db_merkle,
            uncommitted_key_values: Vec::new(),
            dirty_guard: false,
            only_root,
        }
    }

    pub fn get(&self, key: &Key) -> Result<Option<Box<[u8]>>> {
        assert!(
            !self.dirty_guard,
            "Can not read db if set operations have not been committed."
        );

        let version: Option<Value> = match self.db_key_new.get(key.as_ref())? {
            None => None,
            Some(value) => Some(Value::storage_decode(&value)?),
        };

        Ok(version.map(|x| x.value.into_boxed_slice()))
    }

    pub fn set(&mut self, key: &Key, value: Box<[u8]>) {
        self.dirty_guard = true;
        self.uncommitted_key_values.push((key.clone(), value))
    }

    pub fn commit(&mut self, epoch: u64) -> Result<(G1Projective, H256)> {
        let kv_num = self.uncommitted_key_values.len();
        let mut hashes = Vec::with_capacity(kv_num);

        for (position, (key, value)) in self.uncommitted_key_values.drain(..).enumerate() {
            let version: Option<VerInfo> = match self.db_key_new.get(key.as_ref())? {
                None => None,
                Some(value) => Some(Value::storage_decode(&value)?.version),
            };

            let version = self.version_tree.inc_key_ver(&key, version);

            let value = Value {
                value: value.to_vec(),
                version,
                position: EpochPosition {
                    epoch,
                    position: position as u64,
                },
            };

            self.db_key_new.put(key.as_ref(), &value.storage_encode());

            let key_ver_value_hash = keccak(
                &KeyValue {
                    key: key.0,
                    version,
                    value: value.value,
                }
                .storage_encode(),
            );

            hashes.push(key_ver_value_hash);
        }

        // println!("commit position");
        let (amt_root, mut updates) = self.version_tree.commit(epoch, hashes.len() as u64);
        let commitments: Vec<G1<Pairing>> = updates.iter().map(|x| x.1.clone()).collect();
        let affine_commitments = ProjectiveCurve::batch_normalization_into_affine(&commitments);

        for (position, ((tree, _commitment, version), affine_commitment)) in
            updates.drain(..).zip(affine_commitments.iter()).enumerate()
        {
            let name_ver_value_hash = keccak(
                &TreeValue {
                    key: tree.clone(),
                    version_number: version,
                    commitment: *affine_commitment,
                }
                .storage_encode(),
            );
            hashes.push(name_ver_value_hash);
        }

        // println!("commit merkle tree");
        let merkle_root = StaticMerkleTree::dump(
            self.db_pos_value_merkle.clone(),
            epoch,
            hashes,
            self.only_root,
        );

        self.dirty_guard = false;

        Ok((amt_root, merkle_root))
    }

    #[allow(unused_variables, unused_mut)]
    // TODO: for non-existence proof.
    pub fn prove(&mut self, key: &Key) -> Result<Proof> {
        let value = self
            .db_key_new
            .get(key.as_ref())?
            .expect("We only support existent proof");
        let value = Value::storage_decode(&value)?;
        let ver_info = value.version;

        let maybe_value = Some(value.value);

        let assoc_proof = AssociateProof {
            value: maybe_value,
            ver_info,
        };

        // Key value Merkle proof
        let (merkle_epoch, merkle_proof) = self.prove_merkle(value.position)?;

        // AMT Proof
        let tree_name = key.tree_at_level(ver_info.level);
        let index = key.index_at_level(ver_info.level);
        let (commitment, node, amt_proof) = self.prove_amt_node(tree_name.clone(), index);

        let mut level_proofs = VecDeque::with_capacity(ver_info.level as usize + 1);

        level_proofs.push_back(LevelProof {
            merkle_epoch,
            merkle_proof,
            amt_proof,
            commitment,
            node_fr_int: node.as_fr_int(),
            node_version: node.key_versions[ver_info.slot_index as usize],
        });

        for level in (0..ver_info.level).rev() {
            let tree_name = key.tree_at_level(level);
            let index = key.index_at_level(level) as usize;

            let position = self
                .version_tree
                .get_tree_mut(&tree_name)
                .get(index)
                .tree_position;

            let (merkle_epoch, merkle_proof) = self.prove_merkle(position)?;
            let (commitment, node, amt_proof) = self.prove_amt_node(tree_name, index);

            level_proofs.push_front(LevelProof {
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

    pub fn verify<F: Fn(u64) -> H256>(
        key: &Key,
        proof: &Proof,
        epoch_root: F,
        pp: &AMTParams<Pairing>,
    ) -> std::result::Result<(), String> {
        let (assoc_proof, level_proofs) = proof;

        let ver_info = assoc_proof.ver_info;

        // Check the AMT proof
        for (level, level_proof) in level_proofs.iter().enumerate() {
            let amt_index = key.index_at_level(level as u8);
            let amt_proof_verified = AMTree::<AMTConfig>::verify(
                amt_index as usize,
                Fr::<Pairing>::from(level_proof.node_fr_int),
                &level_proof.commitment,
                level_proof.amt_proof.clone(),
                pp,
            );
            if !amt_proof_verified {
                return Err(format!("Incorrect AMT proof at level {}", level));
            }
        }

        // Check Merkle proof in the bottom level.
        if let Some(value) = &assoc_proof.value {
            let bottom_level_proof = &level_proofs[level_proofs.len() - 1];

            let key_ver_value_hash = keccak(
                &KeyValue {
                    key: key.0.clone(),
                    version: ver_info,
                    value: value.to_vec(),
                }
                .storage_encode(),
            );

            let epoch = bottom_level_proof.merkle_epoch;

            let merkle_proof = &bottom_level_proof.merkle_proof;

            let merkle_proof_verified =
                StaticMerkleTree::verify(&epoch_root(epoch), &key_ver_value_hash, merkle_proof);

            if !merkle_proof_verified {
                return Err("Incorrect Merkle proof at level -1".to_string());
            }
        }

        // Check Merkle proof in the rest levels.
        for level in 0..level_proofs.len() - 1 {
            let version = level_proofs[level].node_version;
            let level_proof = &level_proofs[level];
            let tree_name = key.tree_at_level(level as u8 + 1);
            let commitment = level_proofs[level + 1].commitment;

            let key_ver_value_hash = keccak(
                &TreeValue {
                    key: tree_name.clone(),
                    version_number: version,
                    commitment: commitment.into(),
                }
                .storage_encode(),
            );

            let epoch = level_proof.merkle_epoch;
            let merkle_proof = &level_proof.merkle_proof;

            let merkle_proof_verified =
                StaticMerkleTree::verify(&epoch_root(epoch), &key_ver_value_hash, merkle_proof);

            if !merkle_proof_verified {
                println!(
                    "Key {:?}, version {:?}, root{:?} hash {:?}",
                    tree_name.clone(),
                    version,
                    epoch_root(epoch),
                    key_ver_value_hash
                );
                return Err(format!("Incorrect Merkle proof at level {}", level));
            }
        }

        // Check version consistency in the top level.
        {
            let bottom_level_proof = &level_proofs[level_proofs.len() - 1];
            let version_verified = Node::versions_from_fr_int(
                &bottom_level_proof.node_fr_int,
                ver_info.slot_index as usize + 1,
            ) == bottom_level_proof.node_version;
            if !version_verified {
                return Err(format!("Inconsistent version value at level -1"));
            }
        }

        // Check version consistency in the rest levels.
        for level in 0..(level_proofs.len() - 1) {
            let level_proof = &level_proofs[level];
            let version_verified =
                Node::versions_from_fr_int(&level_proof.node_fr_int, 0) == level_proof.node_version;

            if !version_verified {
                return Err(format!("Inconsistent version value at level {}", level));
            }
        }

        Ok(())
    }

    pub fn prove_amt_node(
        &mut self,
        name: TreeName,
        index: usize,
    ) -> (G1<Pairing>, Node, AMTProof<G1<Pairing>>) {
        let tree = self.version_tree.get_tree_mut(&name);

        let commitment = tree.commitment().clone();
        let value = tree.get(index).clone();
        let proof = tree
            .prove(index)
            .expect("Currently, all the nodes are working in full mode");

        return (commitment, value, proof);
    }

    pub fn prove_merkle(&mut self, epoch_pos: EpochPosition) -> Result<(u64, MerkleProof)> {
        let mut tree = StaticMerkleTree::new(self.db_pos_value_merkle.clone(), epoch_pos.epoch);
        let merkle_proof = tree.prove(epoch_pos.position);
        Ok((epoch_pos.epoch, merkle_proof))
    }
}

pub fn new_simple_db<T: TypeUInt>(
    dir: &str,
    only_root: bool,
) -> (SimpleDb, Arc<AMTParams<Pairing>>) {
    use crate::storage::open_database;
    let db = open_database(dir, NUM_COLS);
    let amt_param = Arc::new(AMTParams::<Pairing>::from_dir("./pp", T::USIZE, true));

    (SimpleDb::new(db, amt_param.clone(), only_root), amt_param)
}

#[test]
fn test_simple_db() {
    use crate::crypto::TypeDepths;
    use std::collections::HashMap;

    let (mut db, pp) = new_simple_db::<TypeDepths>("./__test_simple_db", false);

    let mut epoch_root_dict = HashMap::new();

    let mut current_epoch = 0;
    let mut _latest_amt_root = G1Projective::default();

    let verify_key =
        |key: Vec<u8>, value: Vec<u8>, db: &mut SimpleDb, epoch_root_dict: &HashMap<u64, H256>| {
            // println!("Verify key {:?}", key);
            let key = Key(key.to_vec());
            assert_eq!(value, db.get(&key).unwrap().unwrap().into_vec());
            let proof = db.prove(&key).unwrap();
            SimpleDb::verify(&key, &proof, |epoch| epoch_root_dict[&epoch], &pp).unwrap();
        };

    for i in 0..=255 {
        db.set(&Key(vec![1, 2, i, 0]), vec![1, 2, i, 5].into());
        let (amt_root, epoch_root) = db.commit(current_epoch).unwrap();
        _latest_amt_root = amt_root;
        epoch_root_dict.insert(current_epoch, epoch_root);
        current_epoch += 1;
    }

    for i in 0..=40 {
        verify_key(
            vec![1, 2, i, 0],
            vec![1, 2, i, 5],
            &mut db,
            &epoch_root_dict,
        );
    }

    for i in 0..=255 {
        db.set(&Key(vec![1, 2, i, 1]), vec![1, 2, i, 10].into());
        let (amt_root, epoch_root) = db.commit(current_epoch).unwrap();
        _latest_amt_root = amt_root;
        epoch_root_dict.insert(current_epoch, epoch_root);
        current_epoch += 1;
    }

    for i in 0..=40 {
        verify_key(
            vec![1, 2, i, 1],
            vec![1, 2, i, 10],
            &mut db,
            &epoch_root_dict,
        );
    }

    for i in 0..=255 {
        db.set(&Key(vec![1, 2, i, 0]), vec![1, 2, i, 15].into());
        db.set(&Key(vec![1, 2, i, 1]), vec![1, 2, i, 20].into());
        let (amt_root, epoch_root) = db.commit(current_epoch).unwrap();
        _latest_amt_root = amt_root;
        epoch_root_dict.insert(current_epoch, epoch_root);
        current_epoch += 1;
    }

    for i in 0..=40 {
        verify_key(
            vec![1, 2, i, 0],
            vec![1, 2, i, 15],
            &mut db,
            &epoch_root_dict,
        );
        verify_key(
            vec![1, 2, i, 1],
            vec![1, 2, i, 20],
            &mut db,
            &epoch_root_dict,
        );
    }

    std::fs::remove_dir_all("./__test_simple_db").unwrap();
}
