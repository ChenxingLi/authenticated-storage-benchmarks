use algebra_core::{CanonicalDeserialize, CanonicalSerialize};

pub trait StoreByCanonicalSerialize {}

pub trait StorageEncodable {
    fn storage_encode(&self) -> Vec<u8>;
}

pub trait StorageDecodable {
    fn storage_decode(_: &[u8]) -> Self;
}

impl<T: CanonicalSerialize + StoreByCanonicalSerialize> StorageEncodable for T {
    fn storage_encode(&self) -> Vec<u8> {
        let mut serialized = vec![0; self.serialized_size()];
        self.serialize_unchecked(&mut serialized[..]).unwrap();
        serialized
    }
}

impl<T: CanonicalDeserialize + StoreByCanonicalSerialize> StorageDecodable for T {
    fn storage_decode(data: &[u8]) -> Self {
        Self::deserialize_unchecked(data).unwrap()
    }
}

impl StorageEncodable for Vec<u8> {
    fn storage_encode(&self) -> Vec<u8> {
        self.clone()
    }
}

impl StorageDecodable for Vec<u8> {
    fn storage_decode(data: &[u8]) -> Self {
        data.to_vec()
    }
}

// mod error {
//     error_chain! {
//         links {
//         }
//
//         foreign_links {
//         }
//
//         errors {
//             InvalidLength() {
//                 description("Invalid length in decoding.")
//                 display("Invalid length in decoding.")
//             }
//         }
//     }
// }
// pub use error::{Error, Result};
