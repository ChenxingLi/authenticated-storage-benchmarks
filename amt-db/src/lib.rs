// #![allow(dead_code)]
// unused_imports
// non_camel_case_types
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate static_assertions;
#[macro_use]
extern crate log;

extern crate base64;

pub mod amt;
pub mod crypto;
mod enable_log;
pub mod merkle;
pub mod simple_db;
pub mod storage;
pub mod ver_tree;

#[allow(unused)]
use enable_log::*;

#[cfg(not(test))]
const DEPTHS: usize = 16;
#[cfg(test)]
const DEPTHS: usize = 8;
