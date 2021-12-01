#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate static_assertions;

extern crate base64;
#[cfg(test)]
extern crate kvdb_memorydb;

pub mod amt;
pub mod crypto;
mod enable_log;
pub mod merkle;
pub mod multi_layer_amt;
pub mod serde;
pub mod simple_db;
pub mod storage;

#[allow(unused)]
use enable_log::*;

#[cfg(not(any(feature = "medium_amt", feature = "large_amt", feature = "huge_amt")))]
const DEPTHS: usize = 8;
#[cfg(feature = "media_amt")]
const DEPTHS: usize = 12;
#[cfg(feature = "large_amt")]
const DEPTHS: usize = 16;
#[cfg(feature = "huge_amt")]
const DEPTHS: usize = 20;
