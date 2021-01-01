use super::{key::Key, DEPTHS};
use crate::storage::StoreByBytes;
use algebra::{FromBytes, ToBytes};
use std::io::{Error, ErrorKind, Read, Result, Write};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct TreeName(pub(super) usize, pub(super) u128);

impl StoreByBytes for TreeName {}

impl FromBytes for TreeName {
    fn read<R: Read>(mut reader: R) -> Result<Self> {
        let level = u8::read(&mut reader)? as usize;
        let length = u8::read(&mut reader)? as usize;

        if length > 16 {
            let err = Error::new(ErrorKind::InvalidData, "The index length can not exceed 16");
            return Err(err);
        }

        let mut index_bytes = [0u8; 16];
        reader.read_exact(&mut index_bytes[(16 - length)..])?;

        let index = u128::from_be_bytes(index_bytes);

        Ok(TreeName(level, index))
    }
}

impl ToBytes for TreeName {
    fn write<W: Write>(&self, mut writer: W) -> Result<()> {
        let TreeName(level, index) = self;
        let truncates = (index.leading_zeros() / 8) as usize;
        let vec_index = &index.to_be_bytes()[truncates..];

        (*level as u8).write(&mut writer)?;
        ((16 - truncates) as u8).write(&mut writer)?;
        vec_index.write(writer)?;

        Ok(())
    }
}

impl TreeName {
    pub const fn root() -> Self {
        TreeName(0, 0)
    }

    pub fn from_key_level(key: &Key, level: usize) -> Self {
        TreeName(level, key.tree_at_level(level))
    }

    pub fn parent(&self) -> Option<Self> {
        let TreeName(level, index) = self.clone();
        if level == 0 {
            None
        } else {
            Some(TreeName(level - 1, index >> DEPTHS))
        }
    }
}

#[test]
fn test_tree_name_string() {
    use crate::storage::{StorageDecodable, StorageEncodable};

    assert_eq!(TreeName(0, 0).storage_encode(), [0u8, 0u8]);
    assert_eq!(TreeName(0, 0), TreeName::storage_decode(&[0u8, 0u8]));

    assert_eq!(TreeName(1, 0).storage_encode(), [1u8, 0u8]);
    assert_eq!(TreeName(1, 0), TreeName::storage_decode(&[1u8, 0u8]));

    // assert_eq!(TreeName(0, 0).to_bytes(SMALL_DEPTHS), [0u8]);
    //
    // assert_eq!(TreeName(1, 0).to_bytes(SMALL_DEPTHS), [1u8, 0u8]);
    // assert_eq!(TreeName(1, 0).to_bytes(MIDDLE_DEPTHS), [1u8, 0u8, 0u8]);
    // assert_eq!(TreeName(1, 0).to_bytes(LARGE_DEPTHS), [1u8, 0u8, 0u8, 0u8]);
    //
    // assert_eq!(TreeName(1, 1).to_bytes(SMALL_DEPTHS), [1u8, 1u8]);
    // assert_eq!(TreeName(1, 1).to_bytes(MIDDLE_DEPTHS), [1u8, 0u8, 1u8]);
    // assert_eq!(TreeName(1, 1).to_bytes(LARGE_DEPTHS), [1u8, 0u8, 0u8, 1u8]);
    //
    // assert_eq!(
    //     TreeName(2, 1024).to_bytes(LARGE_DEPTHS),
    //     [2u8, 0u8, 0u8, 0u8, 4u8, 0u8]
    // );
}
