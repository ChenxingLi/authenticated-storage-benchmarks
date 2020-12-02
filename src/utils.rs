pub const LEVELS: usize = 6;
pub const LENGTH: usize = 1 << LEVELS;

#[inline]
pub(crate) fn bitreverse(mut n: usize, l: usize) -> usize {
    let mut r = 0;
    for _ in 0..l {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}
