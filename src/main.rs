#![allow(dead_code)]
// non_camel_case_types,unused_imports
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate static_assertions;

extern crate base64;

mod amt;
mod storage;
mod ver_tree;

use bencher::black_box;

fn main() {
    let x = 0;
    black_box(x);
}
