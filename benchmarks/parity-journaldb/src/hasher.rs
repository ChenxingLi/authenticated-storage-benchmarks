#[cfg(feature = "light-hash")]
pub use blake2s_hasher::Blake2sHasher as DBHasher;
#[cfg(not(feature = "light-hash"))]
pub use keccak_hasher::KeccakHasher as DBHasher;
