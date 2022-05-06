use std::collections::VecDeque;
use std::io::Result;
use std::sync::{Arc, RwLock};

use ethereum_types::H256;
use global::Global;
// use hashbrown::hash_map::Entry;
use hashbrown::HashMap;
use keccak_hash::keccak;
use kvdb::{DBKey, DBOp, DBTransaction, KeyValueDB};

use amt_serde_derive::{MyFromBytes, MyToBytes};

use crate::amt::{AMTData, AMTProof, AMTree};
use crate::crypto::{
    export::{Fr, FrInt, G1Aff, G1Projective, G1},
    AMTParams, Pairing, TypeDepths, TypeUInt,
};
use crate::merkle::{MerkleProof, StaticMerkleTree};
use crate::multi_layer_amt::{
    AMTConfig, AMTNodeIndex, EpochPosition, Key, Node, TreeName, VerInfo, VersionTree,
};
use crate::serde::{MyFromBytes, MyToBytes};
use crate::storage::DBColumn;

const COL_VER_TREE: u32 = 0;
const COL_KEY_NEW: u32 = COL_VER_TREE + 1;
const COL_MERKLE: u32 = COL_KEY_NEW + 1;
pub const NUM_COLS: u32 = COL_MERKLE + 1;

pub static INC_KEY_LEVEL_SUM: Global<u64> = Global::INIT;
pub static INC_KEY_COUNT: Global<u64> = Global::INIT;
pub static INC_TREE_COUNT: Global<u64> = Global::INIT;

pub struct AmtDb {
    pub kvdb: Arc<dyn KeyValueDB>,

    version_tree: VersionTree,
    db_key: DBColumn,
    db_merkle: DBColumn,

    cache: RwLock<HashMap<Key, (Option<Value>, bool)>>,
    uncommitted_key_values: Vec<(Key, Box<[u8]>)>,
    dirty_guard: bool,
    only_merkle_root: bool,
}

#[derive(Default, Clone, Debug, MyFromBytes, MyToBytes)]
pub struct TreeValue {
    pub(crate) key: TreeName,
    pub(crate) version_number: u64,
    pub(crate) commitment: G1Aff<Pairing>,
}

#[derive(Default, Clone, Debug, MyFromBytes, MyToBytes)]
pub struct KeyValue {
    pub(crate) key: Vec<u8>,
    pub(crate) version: VerInfo,
    pub(crate) value: Vec<u8>,
}

#[derive(Default, Clone, MyFromBytes, MyToBytes)]
pub struct Value {
    pub(crate) value: Vec<u8>,
    pub(crate) version: VerInfo,
    pub(crate) position: EpochPosition,
}

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

pub type Proof = (AssociateProof, VecDeque<LevelProof>);
pub type AmtRoot = G1Projective;

const EPOCH_NUMBER_KEY: [u8; 2] = [0, 0];

impl AmtDb {
    // The KeyValueDB requires 3 columns.
    pub fn new(
        backend: Arc<dyn KeyValueDB>,
        pp: Arc<AMTParams<Pairing>>,
        only_merkle_root: bool,
        shard_info: Option<(usize, usize)>,
    ) -> Self {
        let db_ver_tree = DBColumn::from_kvdb(backend.clone(), COL_VER_TREE);
        let shard_node = shard_info.map(|(depth, index)| AMTNodeIndex::new(depth, index));
        let version_tree = VersionTree::new(db_ver_tree, pp, shard_node);
        let db_key = DBColumn::from_kvdb(backend.clone(), COL_KEY_NEW);
        let db_merkle = DBColumn::from_kvdb(backend.clone(), COL_MERKLE);
        let kvdb = backend;
        Self {
            kvdb,
            version_tree,
            db_key,
            db_merkle,
            cache: Default::default(),
            uncommitted_key_values: Vec::new(),
            dirty_guard: false,
            only_merkle_root,
        }
    }

    pub fn get(&self, key: &Key) -> Result<Option<Box<[u8]>>> {
        // assert!(
        //     !self.dirty_guard,
        //     "Can not read db if set operations have not been committed."
        // );

        // let mut write_guard = self.cache.write().unwrap();
        // let entry = write_guard.entry(key.clone());

        // let maybe_value = match entry {
        //     Entry::Occupied(entry) => entry.get().0.clone(),
        //     Entry::Vacant(entry) => {
        //         let value = self
        //             .db_key
        //             .get(key.as_ref())?
        //             .map(|x| Value::from_bytes_local(&x).unwrap());
        //         entry.insert((value.clone(), false));
        //         value
        //     }
        // };

        let ans = self
            .db_key
            .get(key.as_ref())?
            .map(|x| Value::from_bytes_local(&x).unwrap())
            .map(|x| x.value.into_boxed_slice());

        Ok(ans)
    }

    pub fn set(&mut self, key: &Key, value: Box<[u8]>) {
        // self.dirty_guard = true;
        // FIXME: write to cache.
        self.uncommitted_key_values.push((key.clone(), value))
    }

    pub fn current_epoch(&self) -> Result<u64> {
        let epoch = self
            .db_merkle
            .get(&EPOCH_NUMBER_KEY)?
            .map_or(0, |x| u64::from_bytes_local(&x).unwrap());
        Ok(epoch)
    }

    pub fn commit(&mut self, _epoch: u64) -> Result<(G1Projective, H256)> {
        let epoch = self.current_epoch()?;

        let kv_num = self.uncommitted_key_values.len();
        let mut hashes = Vec::with_capacity(kv_num);
        let mut write_ops = Vec::with_capacity(kv_num);

        for (position, (key, value)) in self.uncommitted_key_values.drain(..).enumerate() {
            let version: Option<VerInfo> = match self.cache.read().unwrap().get(&key).as_ref() {
                Some(&(value, _)) => value.clone().map(|x| x.version),
                None => match self.db_key.get(key.as_ref())? {
                    None => None,
                    Some(value) => Some(Value::from_bytes_local(&value)?.version),
                },
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
            *INC_KEY_COUNT.lock_mut().unwrap() += 1;
            *INC_KEY_LEVEL_SUM.lock_mut().unwrap() += version.level as u64 + 1;

            write_ops.push(DBOp::Insert {
                col: 0,
                key: key.as_ref().into(),
                value: value.to_bytes_local(),
            });

            let key_ver_value_hash = keccak(
                &KeyValue {
                    key: key.0,
                    version,
                    value: value.value,
                }
                .to_bytes_consensus(),
            );

            hashes.push(key_ver_value_hash);
        }
        self.db_key.write_buffered(DBTransaction { ops: write_ops });

        // println!("commit position");
        let (amt_root, updates) = self.version_tree.commit(epoch, hashes.len() as u64);

        for (tree, version, commitment) in updates.into_iter() {
            let name_ver_value_hash = keccak(
                &TreeValue {
                    key: tree.clone(),
                    version_number: version,
                    commitment,
                }
                .to_bytes_consensus(),
            );
            hashes.push(name_ver_value_hash);
        }

        let merkle_root =
            StaticMerkleTree::dump(self.db_merkle.clone(), epoch, hashes, self.only_merkle_root);
        self.db_merkle.write_buffered(DBTransaction {
            ops: vec![DBOp::Insert {
                col: 0,
                key: DBKey::from_vec(EPOCH_NUMBER_KEY.to_vec()),
                value: (epoch + 1).to_bytes_local(),
            }],
        });

        self.dirty_guard = false;
        self.kvdb.flush()?;
        self.cache.write().unwrap().clear();

        Ok((amt_root, merkle_root))
    }

    // TODO: for non-existence proof.
    pub fn prove(&mut self, key: &Key) -> Result<Proof> {
        let value = self
            .db_key
            .get(key.as_ref())?
            .expect("We only support existent proof");
        let value = Value::from_bytes_local(&value)?;
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
                .to_bytes_consensus(),
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
                .to_bytes_consensus(),
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
        let mut tree = StaticMerkleTree::new(self.db_merkle.clone(), epoch_pos.epoch);
        let merkle_proof = tree.prove(epoch_pos.position);
        Ok((epoch_pos.epoch, merkle_proof))
    }

    pub fn flush_root(&mut self) {
        self.version_tree.flush_all();
        self.kvdb.flush().unwrap();
    }
}

pub fn cached_pp(dir: &str) -> Arc<AMTParams<Pairing>> {
    Arc::new(AMTParams::<Pairing>::from_dir(dir, TypeDepths::USIZE, true))
}

#[test]
fn test_simple_db() {
    use std::collections::HashMap;

    let backend = crate::storage::test_kvdb(NUM_COLS);
    let pp = Arc::new(AMTParams::<Pairing>::from_dir(
        "./pp",
        TypeDepths::USIZE,
        true,
    ));
    let mut db = AmtDb::new(backend, pp.clone(), false, Some((0, 0)));

    let mut epoch_root_dict = HashMap::new();

    let mut current_epoch = 0;
    let mut _latest_amt_root = G1Projective::default();

    let verify_key =
        |key: Vec<u8>, value: Vec<u8>, db: &mut AmtDb, epoch_root_dict: &HashMap<u64, H256>| {
            // println!("Verify key {:?}", key);
            let key = Key(key.to_vec());
            assert_eq!(value, db.get(&key).unwrap().unwrap().into_vec());
            let proof = db.prove(&key).unwrap();
            AmtDb::verify(&key, &proof, |epoch| epoch_root_dict[&epoch], &pp).unwrap();
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
}
