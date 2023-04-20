mod child_ref;
mod nibble;
mod rain_mpt;
#[cfg(test)]
mod tests;
mod trie_node;
mod trie_node_ext;

pub use rain_mpt::MerklePatriciaTree;

fn common_prefix_iter<'a, T: Eq>(a: &'a [T], b: &'a [T]) -> impl Iterator<Item = &'a T> {
    a.iter()
        .zip(b.iter())
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| x.clone())
}

fn add_prefix<T: Copy>(base: &mut Vec<T>, prefix: &[T]) {
    *base = [prefix, &base[..]].concat()
}

#[cfg(feature = "light-hash")]
pub use blake2_hasher::Blake2bHasher as RlpHasher;
#[cfg(not(feature = "light-hash"))]
pub use keccak_hasher::KeccakHasher as RlpHasher;