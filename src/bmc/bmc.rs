
use std::cell::RefCell;
use std::rc::Rc;

use crate::expr::expr::ExprBuilder;
use crate::program::program::*;
use crate::solvers::solver::*;
use crate::symex::symex::*;
use crate::vc::vc::*;
use crate::Config;

pub struct Bmc<'cfg> {
  config: &'cfg Config,
  symex: Symex<'cfg>,
  vc_system: VCSysPtr,
  runtime_solver: Solver<'cfg>,
}

impl<'cfg> Bmc<'cfg> {
  pub fn new(config: &'cfg Config) -> Self {
    let vc_system =
      VCSysPtr::new(RefCell::new(VCSystem::default()));
    let symex =
      Symex::new(&config.program, config.expr_ctx.clone(), vc_system.clone());
    let runtime_solver =
      Solver::new(&config.solver_config);
    Bmc { config, symex, vc_system, runtime_solver }
  }

  pub fn do_bmc(&mut self) {
    if self.config.cli.show_program {
      self.config.program.show();
    }
    while self.symex.can_exec() { self.symex.symex(); }
    self.check_properties();
  }

  fn check_properties(&mut self) {
    if self.config.cli.show_vcc {
      println!("{:?}", self.vc_system.borrow());
    }
    self.generate_smt_formula();

    let presult = self.runtime_solver.check();
    if presult == PResult::PSat {
      println!("Model:\n{}",
        match self.runtime_solver.get_model() {
          Some(m) => format!("{m:?}"),
          None => format!("None"),
        }
      );
    }
  }

  fn generate_smt_formula(&mut self) {
    let ctx = self.config.expr_ctx.clone();

    let mut assumetion = ctx.constant_bool(true);
    let mut assertions = Vec::new();
    
    for vc in self.vc_system.borrow().iter() {
      if vc.is_sliced { continue; }
      match &vc.kind {
        VcKind::Assign(lhs, rhs) => {
          self.runtime_solver.assert_assign(lhs.clone(), rhs.clone());
        },
        VcKind::Assert(_, c) => {
          assertions.push(ctx.implies(assumetion.clone(), c.clone()));
        },
        VcKind::Assume(c) => {
          assumetion = ctx.and(assumetion, c.clone());
          assumetion.simplify();
        },
      }
    }

    self.runtime_solver.assert_expr(
      if assertions.is_empty() { ctx.constant_bool(false) }
      else {
        let mut assertion =
          assertions.into_iter().fold(
            ctx.constant_bool(true),
            |acc, b| ctx.and(acc, b)
          );
        assertion.simplify();
        assertion
      }
    );
  }
}