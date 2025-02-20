
use std::marker::PhantomData;

use crate::expr::expr::Expr;
use crate::program::program::Program;
use crate::vc::vc::*;

use super::config::Config;
use super::smt::smt_conv::*;
use super::z3::z3_conv::*;

pub(crate) enum Result {
  PSat,
  PUnsat,
  PUnknow,
}

pub struct Solver<'ctx> {
  solver: Box<dyn SmtSolver + 'ctx>,
}

impl<'ctx> Solver<'ctx> {
  pub fn new(program: &'ctx Program, config: &'ctx Config) -> Self {
    // TODO: recieve config from cmd
    let mut runtime_solver = Box::new(Z3Conv::new(config.to_z3_ctx()));
    runtime_solver.init(program);
    Solver { solver: runtime_solver }
  }

  pub fn assert_assign(&mut self, lhs: Expr, rhs: Expr) {
    self.solver.assert_assign(lhs, rhs);
  }

  pub fn assert_expr(&mut self, expr: Expr) {
    self.solver.assert_expr(expr);
  }
}