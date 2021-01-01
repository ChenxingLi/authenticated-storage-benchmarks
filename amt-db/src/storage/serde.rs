use algebra_core::{FromBytes, ToBytes};

pub trait StoreByBytes {}
pub trait StoreTupleByBytes {}

pub trait StorageEncodable {
    fn storage_encode(&self) -> Vec<u8>;
}

pub trait StorageDecodable {
    fn storage_decode(data: &[u8]) -> Self;
}

impl<T: ToBytes> StorageEncodable for T
where
    T: StoreByBytes,
{
    fn storage_encode(&self) -> Vec<u8> {
        let mut serialized = Vec::with_capacity(1024);
        self.write(&mut serialized)
            .expect("Write to Vec<u8> should always return Ok(..)");
        serialized.shrink_to_fit();
        serialized
    }
}

impl<T: FromBytes> StorageDecodable for T
where
    T: StoreByBytes,
{
    fn storage_decode(mut data: &[u8]) -> Self {
        FromBytes::read(&mut data).unwrap()
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
            fn storage_decode(mut data: &[u8]) -> Self {
                ($($name::read(&mut data).unwrap()),*)
            }
        }
    };
}

impl_storage_for_tuple!((0=>A),(1=>B));
impl_storage_for_tuple!((0=>A),(1=>B),(2=>C));
