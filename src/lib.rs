#![feature(rustc_private)]
#![feature(assert_matches)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_smir;
extern crate stable_mir;

pub mod bmc;
pub mod config;
mod expr;
mod program;
mod solvers;
mod symbol;
mod symex;
mod vc;
