
use std::hash::Hash;
use std::io::stdout;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::{program::*, ExprCtx};
use crate::state::*;
use crate::symex::*;

pub struct Analyzer {
  ctx: ExprCtx,
  program: Program
}

impl Analyzer {

  pub fn new(program: Program, ctx: ExprCtx) -> Self {
    Analyzer { ctx, program }
  }

  pub fn do_analysis(&mut self) {
    let mut symex =
      Symex::new(&mut self.program, self.ctx.clone());
    symex.run();
  }
}