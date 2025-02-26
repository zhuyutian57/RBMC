
use clap::*;

use crate::NString;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
  /// Source file `.rs`
  pub file: NString,

  /// Show program
  #[arg(long, default_value_t = false)]
  pub show_program: bool,

  /// Show program
  #[arg(long, default_value_t = false)]
  pub show_vcc: bool,

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