
use clap::*;

use crate::NString;

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum SmtStrategy {
  Forward,
  Once,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
  /// Source file `.rs`
  pub file: NString,

  /// Show program
  #[arg(long, default_value_t = false)]
  pub show_program: bool,

  /// Show state
  #[arg(long, default_value_t = false)]
  pub show_states: bool,

  /// Show VCC
  #[arg(long, default_value_t = false)]
  pub show_vcc: bool,

  /// Do not slice VCC
  #[arg(long, default_value_t = false)]
  pub no_slice: bool,

  /// The strategy for invoking SMT solver.
  /// 
  /// `Forward`: stop while an assertion fail.
  /// 
  /// `Once`: encoding all assertions and check only for one time.
  #[clap(value_enum)]
  #[arg(long, default_value_t = SmtStrategy::Forward)]
  pub smt_strategy: SmtStrategy,

  /// Show SMT formula
  #[arg(long, default_value_t = false)]
  pub show_smt: bool,

  /// Show SMT model
  #[arg(long, default_value_t = false)]
  pub show_smt_model: bool,

  /// SMT solver
  #[arg(long, default_value_t = NString::from("z3"))]
  pub solver: NString,
}

impl Cli {
  pub fn rustc_args(&self) -> Vec<String> {
    vec![
      std::env::current_exe().expect("").to_str().unwrap().to_string(),
      self.file.to_string(),
      "-Copt-level=1".to_string(),
      "-Zmir-enable-passes=+ReorderBasicBlocks".to_string()
    ]
  }
}