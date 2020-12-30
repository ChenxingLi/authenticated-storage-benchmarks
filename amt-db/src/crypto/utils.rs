use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub trait TypeUInt {
    const USIZE: usize;
}

pub struct TypeDepths;
impl TypeUInt for TypeDepths {
    const USIZE: usize = DEPTHS;
}

pub const DEPTHS: usize = 6;
pub const LENGTH: usize = 1 << DEPTHS;
pub const IDX_MASK: usize = LENGTH - 1;

pub const ALLOW_RECOMPUTE: bool = true;

pub(crate) fn type_hash<T: Any>() -> String {
    let type_name = std::any::type_name::<T>().to_string();
    let mut s = DefaultHasher::new();
    type_name.hash(&mut s);
    base64::encode(s.finish().to_be_bytes())
}

use algebra::{ConstantSerializedSize, ProjectiveCurve};

// This is an ad-hoc fix due to the upstream crate provides insufficient APIs for projective curve.
// when the const generic stabilized, this function could be a constant function.
pub fn serialize_length<G: ProjectiveCurve>() -> usize {
    let mem_point: usize = std::mem::size_of::<G>();
    let mem_base: usize = std::mem::size_of::<G::BaseField>();

    assert_eq!(mem_point % mem_base, 0);
    let coords: usize = mem_point / mem_base;
    <G::BaseField as ConstantSerializedSize>::UNCOMPRESSED_SIZE * coords
}

#[test]
fn test_serialize_length() {
    use algebra::bls12_381::G1Projective as G1;
    use algebra::One;
    use algebra::ToBytes;

    let sample: G1 = G1::prime_subgroup_generator();
    let mut result: Vec<u8> = Vec::new();
    sample.write(&mut result).unwrap();

    assert_eq!(serialize_length::<G1>(), result.len());
}
