#![allow(dead_code, non_camel_case_types)]
//unused_imports,
#[macro_use]
extern crate error_chain;

mod amt;
mod db;

use bencher::black_box;

fn main() {
    let x = 0;
    black_box(x);
}
