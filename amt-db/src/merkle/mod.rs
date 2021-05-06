use crate::storage::{DBAccess, FlattenArray, KvdbRocksdb};
use cfx_types::H256;
use keccak_hash::{keccak, KECCAK_EMPTY};

pub struct StaticMerkleTree {
    db: KvdbRocksdb,
    epoch: u64,
    data: DBAccess<usize, H256, FlattenArray>,
    root: H256,
    depth: u32,
}

pub type MerkleProof = (Vec<H256>, u64);

fn combine_hash(a: &H256, b: &H256) -> H256 {
    let mut input = a.0.to_vec();
    input.extend_from_slice(&b.0);
    let answer = keccak(&input);
    answer
}

impl StaticMerkleTree {
    pub fn new(db: KvdbRocksdb, epoch: u64) -> Self {
        let mut backend: DBAccess<usize, H256, FlattenArray> =
            DBAccess::new(epoch.to_be_bytes().into(), db.clone());
        let depth = backend.get(&0).to_low_u64_be() as u32;
        let root = backend.get(&1).clone();
        Self {
            db,
            epoch,
            data: backend,
            depth,
            root,
        }
    }

    pub fn root(&self) -> &H256 {
        &self.root
    }

    pub fn prove(&mut self, position: u64) -> MerkleProof {
        let mut proofs = Vec::with_capacity(self.depth as usize);
        for depth in (1..=self.depth).rev() {
            let height = self.depth - depth;
            let index = (1 << depth) | ((position >> height) ^ 1) as usize;
            let mut answer = self.data.get(&index).clone();
            if answer == Default::default() {
                answer = KECCAK_EMPTY
            };
            proofs.push(answer);
        }
        return (proofs, position);
    }

    pub fn verify(root: &H256, hash: &H256, proof: &MerkleProof) -> bool {
        let (merkle_path, pos) = proof;
        let mut current_hash = hash.clone();
        for (index, proof) in merkle_path.iter().enumerate() {
            let right_append = (*pos >> index) % 2 == 0;
            current_hash = if right_append {
                combine_hash(&current_hash, proof)
            } else {
                combine_hash(proof, &current_hash)
            };
        }
        current_hash == *root
    }

    pub fn dump<'a>(db: KvdbRocksdb, epoch: u64, data: Vec<H256>) -> H256 {
        let length = data.len();
        let depth = length.next_power_of_two().trailing_zeros();

        let mut backend: DBAccess<usize, H256, FlattenArray> =
            DBAccess::new(epoch.to_be_bytes().into(), db.clone());

        let mut this_level = data;

        for level in (0..=depth).rev() {
            for (i, hash) in this_level.iter().enumerate() {
                *backend.get_mut(&((1 << level) + i)) = hash.clone();
                // println!("pos {:02}, value {:?}", ((1 << level) + i), hash.clone());
            }
            if this_level.len() % 2 != 0 {
                this_level.push(KECCAK_EMPTY);
            }
            this_level = this_level
                .chunks(2)
                .map(|x| combine_hash(&x[0], &x[1]))
                .collect();
        }

        *backend.get_mut(&0) = H256::from_low_u64_be(depth as u64);

        let root = backend.get(&1).clone();

        backend.flush();

        return root;
    }
}

#[test]
fn test_static_merkle_tree() {
    let db = crate::storage::open_col("./__test_static_merkle_tree", 0);
    for epoch in 1u64..=32 {
        let data: Vec<H256> = (0..epoch)
            .map(|x| H256::from_low_u64_be(x + 65536))
            .collect();
        let root = StaticMerkleTree::dump(db.clone(), epoch, data);

        let mut tree = StaticMerkleTree::new(db.clone(), epoch);
        assert_eq!(root, tree.root().clone());
        for i in 0..epoch {
            let proof = tree.prove(i);
            assert!(
                StaticMerkleTree::verify(&root, &H256::from_low_u64_be(i + 65536), &proof),
                "fail proof at tree {} pos {}",
                epoch,
                i
            );
        }
    }
    ::std::fs::remove_dir_all("./__test_static_merkle_tree").unwrap();
}
