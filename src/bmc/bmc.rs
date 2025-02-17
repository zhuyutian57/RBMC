
use std::cell::RefCell;
use std::rc::Rc;

use crate::program::program::*;
use crate::solvers::solver::*;
use crate::symex::symex::*;
use crate::vc::vc::*;
use crate::Config;

pub struct Bmc<'bmc> {
  program: &'bmc Program,
  config: &'bmc Config,
  symex: Symex<'bmc>,
  vc_system: VCSysPtr,
  runtime_solver: Solver<'bmc>,
}

impl<'bmc> Bmc<'bmc> {
  pub fn new(program: &'bmc Program, config: &'bmc Config) -> Self {
    let vc_system =
      VCSysPtr::new(RefCell::new(VCSystem::default()));
    let symex =
      Symex::new(program, config.expr_ctx(), vc_system.clone());
    let runtime_solver =
      Solver::new(program, config.solver_config());
    Bmc { program, config, symex, vc_system, runtime_solver }
  }

  pub fn do_bmc(&mut self) {
    while self.symex.can_exec() { self.symex.symex(); }
    println!("{:?}", self.vc_system);
    self.check_properties();
  }

  fn check_properties(&self) {
    
  }
}