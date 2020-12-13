use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub const DEPTHS: usize = 6;
pub const LENGTH: usize = 1 << DEPTHS;
pub const IDX_MASK: usize = LENGTH - 1;

pub const ALLOW_RECOMPUTE: bool = true;

#[inline]
pub(crate) fn bitreverse(mut n: usize, l: usize) -> usize {
    let mut r = 0;
    for _ in 0..l {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}

pub(crate) fn type_hash<T: Any>() -> String {
    let type_name = std::any::type_name::<T>().to_string();
    let mut s = DefaultHasher::new();
    type_name.hash(&mut s);
    base64::encode(s.finish().to_be_bytes())
}
