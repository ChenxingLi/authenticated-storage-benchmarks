// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub enum LedgerKey<'a> {
//     AMTNodeKey {
//         amt_name: &'a [u8],
//         chunk_index: usize,
//     },
//     AMTCommitmentKey {
//         amt_name: &'a [u8],
//     },
// }
//
// impl<'a> LedgerKey<'a> {
//     pub fn to_key_bytes(&self) -> Vec<u8> {
//         let mut x = Vec::new();
//         match self {
//             LedgerKey::DataKey { key, version } => {
//                 x.extend_from_slice(&[0]);
//                 x.extend_from_slice(key);
//                 x.extend_from_slice()
//             }
//         }
//         x
//     }
// }
