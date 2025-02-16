
#![feature(rustc_private)]
#![feature(assert_matches)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
#[macro_use]
extern crate rustc_smir;
extern crate stable_mir;

use rustc_smir::{run, rustc_internal};
use std::cell::RefCell;
use std::ops::ControlFlow;
use std::process::ExitCode;
use stable_mir::*;

mod analysis;
mod config;
mod expr;
mod program;
mod solvers;
mod symbol;
mod symex;
mod vc;

use crate::analysis::Analyzer;
use crate::config::Config;
use crate::expr::context::*;
use crate::program::program::Program;
use crate::symbol::nstring::NString;

/// This is a wrapper that can be used to replace rustc.
fn main() -> ExitCode {
  let mut rustc_args: Vec<_> = std::env::args().into_iter().collect();
  rustc_args.push("-Copt-level=1".to_string());
  rustc_args.push("-Zmir-enable-passes=+ReorderBasicBlocks".to_string());
  let result = run!(rustc_args, start_demo);
  match result {
    Ok(_) | Err(CompilerError::Skipped | CompilerError::Interrupted(_)) => ExitCode::SUCCESS,
    _ => ExitCode::FAILURE,
  }
}

fn start_demo() -> ControlFlow<()> {
  let config = Config::new();

  let _crate = NString::from(stable_mir::local_crate().name);
  let items = stable_mir::all_local_items();
  let program = Program::new(_crate, items);

  let mut analyzer = Analyzer::new(program, config);
  
  analyzer.do_analysis();

  ControlFlow::Break(())
}