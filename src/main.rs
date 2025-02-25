
#![feature(rustc_private)]
#![feature(assert_matches)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
#[macro_use]
extern crate rustc_smir;
extern crate stable_mir;

use clap::Parser;
use rustc_smir::{run, rustc_internal};
use std::cell::RefCell;
use std::ops::ControlFlow;
use std::process::ExitCode;
use stable_mir::*;

mod bmc;
mod config;
mod expr;
mod program;
mod solvers;
mod symbol;
mod symex;
mod vc;

use crate::bmc::bmc::Bmc;
use crate::config::cli::*;
use crate::config::config::Config;
use crate::expr::context::*;
use crate::program::program::Program;
use crate::symbol::nstring::NString;

fn main() -> ExitCode {
  let cli = Cli::parse();
  match run!(cli.rustc_args(), || stable_mir_bmc(cli)) {
    Ok(_) | Err(CompilerError::Skipped | CompilerError::Interrupted(_))
      => ExitCode::SUCCESS,
    _ => ExitCode::FAILURE,
  }
}

fn stable_mir_bmc(cli: Cli) -> ControlFlow<()> {
  let config = Config::new(&cli);
  let mut bmc = Bmc::new(&config);
  bmc.do_bmc();

  ControlFlow::Break(())
}