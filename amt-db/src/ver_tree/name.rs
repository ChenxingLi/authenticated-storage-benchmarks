use crate::crypto::export::{CanonicalDeserialize, CanonicalSerialize, SerializationError};
use crate::impl_storage_from_canonical;
use crate::storage::{StorageDecodable, StorageEncodable};
use std::io::{Read, Write};

#[derive(Default, Debug, Clone, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct TreeName(pub(super) Vec<u32>);

impl CanonicalSerialize for TreeName {
    fn serialize<W: Write>(&self, mut writer: W) -> Result<(), SerializationError> {
        let length = self.0.len() as u8;
        length.serialize(&mut writer)?;
        for item in &self.0 {
            item.serialize(&mut writer)?;
        }
        Ok(())
    }

    fn serialized_size(&self) -> usize {
        1 + 4 * self.0.len()
    }
}

impl CanonicalDeserialize for TreeName {
    fn deserialize<R: Read>(mut reader: R) -> Result<Self, SerializationError> {
        let len = u8::deserialize(&mut reader)?;
        let mut values = Vec::new();
        for _ in 0..len {
            values.push(u32::deserialize(&mut reader)?);
        }
        Ok(TreeName(values))
    }
}

impl_storage_from_canonical!(TreeName);

impl TreeName {
    pub const fn root() -> Self {
        TreeName(Vec::new())
    }

    pub fn level_index(&self) -> Option<u32> {
        self.0.last().cloned()
    }

    pub fn child(&self, index: u32) -> Self {
        let mut answer = self.clone();
        answer.0.push(index);
        answer
    }

    pub fn parent(&self) -> Option<Self> {
        let mut answer = self.clone();
        let top_element = answer.0.pop();
        if top_element.is_none() {
            None
        } else {
            Some(answer)
        }
    }
}

#[test]
fn test_tree_name_string() {
    assert_eq!(TreeName(vec![]).storage_encode(), [0u8]);

    assert_eq!(
        TreeName(vec![1]).storage_encode(),
        [1u8, 1u8, 0u8, 0u8, 0u8]
    );

    assert_eq!(
        TreeName::storage_decode(&TreeName(vec![1, 2, 3]).storage_encode()).unwrap(),
        TreeName(vec![1, 2, 3])
    );
}
