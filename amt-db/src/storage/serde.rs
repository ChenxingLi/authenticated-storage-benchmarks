use crate::crypto::export::{
    CanonicalDeserialize, CanonicalSerialize, FromBytes, G1Projective, ToBytes,
};
use keccak_hash::H256;

pub trait StoreByBytes {}

pub trait StorageEncodable {
    fn storage_encode(&self) -> Vec<u8>;
}

pub trait StorageDecodable
where
    Self: Sized,
{
    fn storage_decode(data: &[u8]) -> Result<Self>;
}

#[macro_export]
macro_rules! impl_storage_from_canonical {
    ( $name:ident<$s:ident> where $t:ident: $trait_name:ident ) => {
        impl<$t: $trait_name> StorageEncodable for $name<$s> {
            fn storage_encode(&self) -> Vec<u8> {
                let mut serialized = Vec::with_capacity(self.serialized_size());
                self.serialize_unchecked(&mut serialized).unwrap();
                serialized
            }
        }

        impl<$t: $trait_name> StorageDecodable for $name<$s> {
            fn storage_decode(data: &[u8]) -> crate::storage::serde::Result<Self> {
                Ok(Self::deserialize_unchecked(data)?)
            }
        }
    };
    ( $name:ident ) => {
        impl StorageEncodable for $name {
            fn storage_encode(&self) -> Vec<u8> {
                let mut serialized = Vec::with_capacity(self.serialized_size());
                self.serialize_unchecked(&mut serialized).unwrap();
                serialized
            }
        }

        impl StorageDecodable for $name {
            fn storage_decode(data: &[u8]) -> crate::storage::serde::Result<Self> {
                Ok(Self::deserialize_unchecked(data)?)
            }
        }
    };
}

impl StorageEncodable for H256 {
    fn storage_encode(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl StorageDecodable for H256 {
    fn storage_decode(data: &[u8]) -> Result<Self> {
        Ok(H256::from_slice(data))
    }
}

// I can not implement `impl_storage_from_canonical!(G1Projective)` here. So I implement for
// `G1Projective` instead.
// See https://github.com/arkworks-rs/algebra/issues/185 for more details.
impl_storage_from_canonical!(G1Projective);

impl<T: ToBytes + StoreByBytes> StorageEncodable for T {
    fn storage_encode(&self) -> Vec<u8> {
        let mut serialized = Vec::with_capacity(1024);
        self.write(&mut serialized)
            .expect("Write to Vec<u8> should always return Ok(..)");
        serialized.shrink_to_fit();
        serialized
    }
}

impl<T: FromBytes + StoreByBytes> StorageDecodable for T {
    fn storage_decode(mut data: &[u8]) -> Result<Self> {
        Ok(FromBytes::read(&mut data)?)
    }
}

use error_chain;
error_chain! {
    links {

    }

    foreign_links {
        AlgebraSerializeErr(crate::crypto::export::SerializationError);
        StdIoErr(std::io::Error);
    }
}
