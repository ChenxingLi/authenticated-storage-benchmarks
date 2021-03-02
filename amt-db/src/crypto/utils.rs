use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

pub trait TypeUInt: Copy + Eq + Hash + Debug + Sized {
    const USIZE: usize;
}

const DEPTHS: usize = 8;

#[macro_export]
macro_rules! type_uint {
    ( $(#[$attr:meta])* $visibility:vis struct $name:ident ($num:tt); ) => {
        $(#[$attr])*
		#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
		$visibility struct $name;

		impl TypeUInt for $name {
            const USIZE: usize = $num;
        }
    };
}

type_uint! {
    pub struct TypeDepths(DEPTHS);
}

pub const ALLOW_RECOMPUTE: bool = true;

pub(crate) fn type_hash<T: Any>() -> String {
    let type_name = std::any::type_name::<T>().to_string();
    let mut s = DefaultHasher::new();
    type_name.hash(&mut s);
    base64::encode(s.finish().to_be_bytes())
}

use crate::crypto::export::{CanonicalSerialize, ProjectiveCurve};

// This is an ad-hoc fix due to the upstream crate provides insufficient APIs for projective curve.
// when the const generic stabilized, this function could be a constant function.
pub fn serialize_length<G: ProjectiveCurve>() -> usize {
    let mem_point: usize = std::mem::size_of::<G>();
    let mem_base: usize = std::mem::size_of::<G::BaseField>();

    assert_eq!(mem_point % mem_base, 0);
    let coords: usize = mem_point / mem_base;
    (G::BaseField::default()).uncompressed_size() * coords
}

#[test]
fn test_serialize_length() {
    use crate::crypto::export::{G1Projective as G1, ToBytes};

    let sample: G1 = G1::prime_subgroup_generator();
    let mut result: Vec<u8> = Vec::new();
    sample.write(&mut result).unwrap();

    assert_eq!(serialize_length::<G1>(), result.len());
}
