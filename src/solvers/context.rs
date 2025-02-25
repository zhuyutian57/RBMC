
use crate::config::cli::Cli;

pub(crate) enum SolverCtx {
  Z3(z3::Context),
}

impl SolverCtx {
  pub fn new(cli: &Cli) -> Self {
    if cli.solver == "z3" {
      SolverCtx::Z3(z3::Context::new(&z3::Config::new()))
    } else {
      panic!("Not support for solve {:?}", cli.solver)
    }
  }

  pub fn to_z3_ctx(&self) -> &z3::Context {
    match self { SolverCtx::Z3(ctx) => ctx, }
  }
}