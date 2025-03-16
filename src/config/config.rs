
use std::cell::RefCell;

use crate::expr;
use crate::program::program::Program;
use crate::solvers::context::SolverCtx;
use crate::ExprCtx;
use crate::solvers;
use crate::NString;

use super::cli::Cli;

pub(crate) struct Config {
  pub(crate) cli: Cli,
  pub(crate) program: Program,
  pub(crate) expr_ctx: ExprCtx,
  pub(crate) solver_config: solvers::context::SolverCtx,
}

impl Config {
  pub fn new(cli: Cli) -> Self {
    // Get stable mir
    let program =
      Program::new(stable_mir::local_crate());
    
    // Context for managing Expr
    let expr_ctx =
      ExprCtx::new(RefCell::new(expr::context::Context::new()));

    // Initilized solver
    let solver_config = SolverCtx::new(&cli);

    Config { cli, program, expr_ctx, solver_config }
  }
}