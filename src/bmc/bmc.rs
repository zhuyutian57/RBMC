
use std::cell::RefCell;
use std::rc::Rc;

use crate::expr::expr::ExprBuilder;
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
    self.check_properties();
  }

  fn check_properties(&mut self) {
    self.generate_smt_formula();
  }

  fn generate_smt_formula(&mut self) {
    let ctx = self.config.expr_ctx();

    let mut cond = ctx.constant_bool(true);
    let mut assertions = Vec::new();
    
    for vc in self.vc_system.borrow().iter() {
      if vc.is_sliced { continue; }
      match &vc.kind {
        VcKind::Assign(lhs, rhs) => {
          self.runtime_solver.assert_assign(lhs.clone(), rhs.clone());
        },
        VcKind::Assert(c) => {
          assertions.push(ctx.implies(cond.clone(), c.clone()));
        },
        VcKind::Assume(c) => {
          cond = ctx.and(cond, c.clone());
          cond.simplify();
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