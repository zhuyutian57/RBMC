
use std::marker::PhantomData;

use super::config::Config;
use super::smt::smt_conv::*;
use super::z3::z3_conv::*;

pub(crate) enum Result {
  PSat,
  PUnsat,
  PUnknow,
}

pub struct Solver<'ctx> {
  solver: Box<dyn Decide + 'ctx>,
}

impl<'ctx> Solver<'ctx> {
  pub fn new(config: &'ctx Config) -> Self {
    
    Solver { solver: Box::new(Z3Conv::new(config.to_z3_ctx())) }
  }
}