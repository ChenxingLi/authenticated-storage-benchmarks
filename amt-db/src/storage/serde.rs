use algebra_core::{FromBytes, ToBytes};
use keccak_hash::H256;

pub trait StoreByBytes {}
pub trait StoreTupleByBytes {}

pub trait StorageEncodable {
    fn storage_encode(&self) -> Vec<u8>;
}

pub trait StorageDecodable
where
    Self: Sized,
{
    fn storage_decode(data: &[u8]) -> Result<Self>;
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

macro_rules! impl_storage_for_tuple {
    ($( ($idx:tt => $name:ident) ),* ) => {
        impl<$($name:ToBytes),*> StorageEncodable for ($($name),* ) where ($($name),* ): StoreTupleByBytes{
            fn storage_encode(&self) -> Vec<u8> {
                let mut serialized = Vec::with_capacity(1024);
                $(self.$idx.write(&mut serialized)
                    .expect("Write to Vec<u8> should always return Ok(..)");)*
                serialized.shrink_to_fit();
                serialized
            }
        }

        impl<$($name:FromBytes),*> StorageDecodable for ($($name),*) where ($($name),* ): StoreTupleByBytes{
            fn storage_decode(mut data: &[u8]) -> Result<Self> {
                Ok(($($name::read(&mut data)?),*))
            }
        }
    };
}

impl_storage_for_tuple!((0=>A),(1=>B));
impl_storage_for_tuple!((0=>A),(1=>B),(2=>C));
impl_storage_for_tuple!((0=>A),(1=>B),(2=>C),(2=>D));
impl_storage_for_tuple!((0=>A),(1=>B),(2=>C),(3=>D),(4=>E));

use error_chain;
error_chain! {
    links {

    }

    foreign_links {
        AlgebraSerializeErr(algebra_core::serialize::SerializationError);
        StdIoErr(std::io::Error);
    }
}
