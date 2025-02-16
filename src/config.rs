
use std::cell::RefCell;

use crate::expr;
use crate::solvers::solver;
use crate::ExprCtx;
use crate::solvers;


pub(crate) struct Config {
  expr_ctx: ExprCtx,
  solver_config: solvers::config::Config,
}

impl Config {
  pub fn new() -> Self {
    let expr_ctx =
      ExprCtx::new(RefCell::new(expr::context::Context::new()));
    let solver_config = solvers::config::Config::new("z3".to_string());
    Config { expr_ctx, solver_config }
  }

  pub fn expr_ctx(&self) -> ExprCtx { self.expr_ctx.clone() }

  pub fn solver_config(&self) -> &solvers::config::Config {
    &self.solver_config
  }
}