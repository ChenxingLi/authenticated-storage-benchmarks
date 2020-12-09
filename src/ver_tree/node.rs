use super::Key;
use crate::amt::{
    paring_provider::{Fr as FrGeneric, FrInt as FrIntGeneric, Pairing},
    AMTData,
};
use algebra::{
    BigInteger, CanonicalDeserialize, CanonicalSerialize, FpParameters, PrimeField, Read,
    SerializationError, Write,
};

type Fr = FrGeneric<Pairing>;
type FrInt = FrIntGeneric<Pairing>;

#[allow(dead_code)]
fn const_assert() {
    const CAPACITY: u32 = <Fr as PrimeField>::Params::CAPACITY;
    const_assert!(CAPACITY > 31 * 8);
}

#[derive(Clone)]
pub enum VerNode {
    Empty,
    Squeeze(Vec<u32>, Vec<Key>, FrInt),
    NodeComm(u64, Key),
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
            VerNode::Squeeze(versions, keys, _fr_int) => {
                SQUEEZE_NODE.serialize(&mut writer)?;
                versions.serialize(&mut writer)?;
                keys.serialize(&mut writer)?;
            }
            VerNode::NodeComm(version, key) => {
                NODE_COMM.serialize(&mut writer)?;
                version.serialize(&mut writer)?;
                key.serialize(&mut writer)?;
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
            VerNode::Squeeze(versions, keys, _fr_int) => {
                versions.serialized_size() + keys.serialized_size()
            }
            VerNode::NodeComm(version, key) => version.serialized_size() + key.serialized_size(),
            VerNode::TreeComm(version) => version.serialized_size(),
        }
    }
}

fn make_fr_int(versions: &Vec<u32>) -> FrInt {
    let mut summation = FrInt::from(0);
    for (idx, ver) in versions.iter().enumerate() {
        let mut fr_int = FrInt::from(*ver as u64);
        fr_int.muln(31 * idx as u32);
        summation.add_nocarry(&fr_int);
    }
    summation
}

impl CanonicalDeserialize for VerNode {
    fn deserialize<R: Read>(reader: R) -> Result<Self, SerializationError> {
        let result = Self::deserialize_uncompressed(reader);
        if let Ok(VerNode::Squeeze(versions, keys, _)) = &result {
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
                let fr_int = make_fr_int(&versions);
                VerNode::Squeeze(versions, keys, fr_int)
            }
            NODE_COMM => {
                let version = u64::deserialize(&mut reader)?;
                let key = Key::deserialize(&mut reader)?;
                VerNode::NodeComm(version, key)
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

impl AMTData<Fr> for VerNode {
    fn as_fr_int(&self) -> FrInt {
        match self {
            VerNode::Empty => FrInt::from(0),
            VerNode::Squeeze(_versions, _keys, fr_int) => fr_int.clone(),
            VerNode::NodeComm(version, _key) => FrInt::from(*version),
            VerNode::TreeComm(version) => FrInt::from(*version),
        }
    }
}
