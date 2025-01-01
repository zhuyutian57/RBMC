//! Small utility that print some information about a crate.

#![feature(rustc_private)]
#![feature(assert_matches)]

// Maybe deprecated util LazyCell is stable
#![feature(lazy_get)]

extern crate rustc_driver;
extern crate rustc_interface;
#[macro_use]
extern crate rustc_smir;
extern crate stable_mir;


use rustc_smir::{run, rustc_internal};
use std::cell::{Ref, RefCell};
use std::ops::ControlFlow;
use std::process::{ExitCode, Termination};
use std::rc::Rc;
use stable_mir::*;
use stable_mir::mir::*;
use stable_mir::target::*;

mod analysis;
mod expr;
mod program;
mod symbol;
mod symex;
mod vc;

use crate::analysis::Analyzer;
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
  let ctx = ExprCtx::new(RefCell::new(Context::new()));

  let _crate = NString::from(stable_mir::local_crate().name);
  let items = stable_mir::all_local_items();
  let target = MachineInfo::target();
  let program = Program::new(_crate, target, items, ctx.clone());

  let mut analyzer = Analyzer::new(program, ctx);
  
  analyzer.do_analysis();

  ControlFlow::Break(())
}