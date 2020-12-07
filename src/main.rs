#![allow(dead_code)]
//unused_imports, non_camel_case_types
#[macro_use]
extern crate error_chain;

extern crate base64;

mod amt;
mod db;

use bencher::black_box;

fn main() {
    let x = 0;
    black_box(x);
}
