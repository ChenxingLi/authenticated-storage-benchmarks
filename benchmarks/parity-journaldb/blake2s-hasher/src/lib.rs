extern crate blake2;
extern crate ethereum_types;
extern crate hash_db;
extern crate plain_hasher;

use blake2::{Blake2s256, Digest};
use ethereum_types::H256;
use hash_db::Hasher;
use plain_hasher::PlainHasher;

/// Concrete `Hasher` impl for the Keccak-256 hash
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Blake2sHasher;
impl Hasher for Blake2sHasher {
    type Out = H256;
    type StdHasher = PlainHasher;
    const LENGTH: usize = 32;
    fn hash(x: &[u8]) -> Self::Out {
        let mut hasher = Blake2s256::new();
        hasher.update(x);
        let digest = hasher.finalize();
        let mut answer = H256::zero();
        answer.0[..].copy_from_slice(&digest);
        answer
    }
}
