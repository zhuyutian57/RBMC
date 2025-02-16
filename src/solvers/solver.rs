
use std::marker::PhantomData;

use super::config::Config;
use super::smt::smt_conv::*;
use super::z3::z3_conv::*;

pub struct Solver<'ctx> {
  solver: Box<dyn SolverApi + 'ctx>,
}

impl<'ctx> Solver<'ctx> {
  pub fn new(config: &'ctx Config) -> Self {
    
    Solver {
      solver: Box::new(Z3Conv::new(config.to_z3_ctx()))
    }
  }
}