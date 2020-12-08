use algebra::{CanonicalDeserialize, CanonicalSerialize, SerializationError};
use std::io::{Read, Write};

pub type Key = Vec<u8>;

#[derive(Clone)]
pub enum VerNode {
    Empty,
    Squeeze(Vec<u32>, Vec<Key>),
    NodeComm(u64),
    TreeComm(u64),
}

const EMPTY_NODE: u8 = 0;
const SQUEEZE_NODE: u8 = 1;
const NODE_COMM: u8 = 2;
const TREE_COMM: u8 = 3;

impl Default for VerNode {
    fn default() -> Self {
        VerNode::Empty
    }
}

impl CanonicalSerialize for VerNode {
    fn serialize<W: Write>(&self, mut writer: W) -> Result<(), SerializationError> {
        match self {
            VerNode::Empty => EMPTY_NODE.serialize(&mut writer)?,
            VerNode::Squeeze(versions, keys) => {
                SQUEEZE_NODE.serialize(&mut writer)?;
                versions.serialize(&mut writer)?;
                keys.serialize(&mut writer)?;
            }
            VerNode::NodeComm(version) => {
                NODE_COMM.serialize(&mut writer)?;
                version.serialize(&mut writer)?;
            }
            VerNode::TreeComm(version) => {
                TREE_COMM.serialize(&mut writer)?;
                version.serialize(&mut writer)?;
            }
        }

        Ok(())
    }

    fn serialized_size(&self) -> usize {
        1 + match self {
            VerNode::Empty => 0,
            VerNode::Squeeze(versions, keys) => versions.serialized_size() + keys.serialized_size(),
            VerNode::NodeComm(version) | VerNode::TreeComm(version) => version.serialized_size(),
        }
    }
}

impl CanonicalDeserialize for VerNode {
    fn deserialize<R: Read>(reader: R) -> Result<Self, SerializationError> {
        let result = Self::deserialize_uncompressed(reader);
        if let Ok(VerNode::Squeeze(versions, keys)) = &result {
            if versions.len() != keys.len() || versions.len() > 8 {
                return Err(SerializationError::InvalidData);
            }
            for (version, _key) in versions.iter().zip(keys.iter()) {
                if *version != 0 || *version > i32::MAX as u32 {
                    return Err(SerializationError::InvalidData);
                }
            }
        }
        result
    }

    fn deserialize_unchecked<R: Read>(mut reader: R) -> Result<Self, SerializationError> {
        let node_type = u8::deserialize(&mut reader)?;
        let result = match node_type {
            EMPTY_NODE => VerNode::Empty,
            SQUEEZE_NODE => {
                let versions = Vec::<u32>::deserialize(&mut reader)?;
                let keys = Vec::<Key>::deserialize(&mut reader)?;
                VerNode::Squeeze(versions, keys)
            }
            NODE_COMM => {
                let version = u64::deserialize(&mut reader)?;
                VerNode::NodeComm(version)
            }
            TREE_COMM => {
                let version = u64::deserialize(&mut reader)?;
                VerNode::TreeComm(version)
            }
            _ => {
                return Err(SerializationError::InvalidData);
            }
        };
        Ok(result)
    }
}
