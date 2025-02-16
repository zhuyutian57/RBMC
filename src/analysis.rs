
use crate::program::program::*;
use crate::symex::symex::*;
use crate::Config;

pub struct Analyzer {
  config: Config,
  program: Program
}

impl Analyzer {

  pub fn new(program: Program, config: Config) -> Self {
    Analyzer { config, program }
  }

  pub fn do_analysis(&mut self) {
    let mut symex =
      Symex::new(&mut self.program, &self.config);
    symex.run();
  }
}