
use crate::expr::expr::Expr;
use crate::program::program::Program;
use crate::vc::vc::*;

use super::context::SolverCtx;
use super::smt::smt_conv::*;
use super::z3::z3_conv::*;

#[derive(Debug, Clone, Copy)]
pub(crate) enum PResult {
  PSat,
  PUnsat,
  PUnknow,
}

pub struct Solver<'ctx> {
  runtime_solver: Box<dyn SmtSolver + 'ctx>,
}

impl<'ctx> Solver<'ctx> {
  pub fn new(solver_ctx: &'ctx SolverCtx) -> Self {
    
    let mut runtime_solver =
      match solver_ctx {
        SolverCtx::Z3(ctx)
          => Box::new(Z3Conv::new(ctx)),
      };
    runtime_solver.init();
    Solver { runtime_solver }
  }

  pub fn check(&self) -> PResult {
    self.runtime_solver.check()
  }

  pub fn assert_assign(&mut self, lhs: Expr, rhs: Expr) {
    self.runtime_solver.assert_assign(lhs, rhs);
  }

  pub fn assert_expr(&mut self, expr: Expr) {
    self.runtime_solver.assert_expr(expr);
  }
}