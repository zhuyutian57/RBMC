
use clap::*;

use crate::NString;

#[derive(clap::ValueEnum, Debug, Default, Clone, Copy)]
pub enum SmtStrategy {
  #[default]
  Forward,
  Once,
}

#[derive(clap::ValueEnum, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DisplayState {
  #[default]
  None,
  BB,
  Statement,
  Terminator,
  All,
}

#[derive(Parser, Debug, Default)]
#[command(version, about, long_about = None)]
pub struct Cli {
  /// Source file `.rs`
  #[arg(default_value_t = NString::EMPTY)]
  pub file: NString,

  /// Show program
  #[arg(long, default_value_t = false)]
  pub show_program: bool,

  /// Show state
  #[clap(value_enum)]
  #[arg(long, default_value_t = DisplayState::None)]
  pub show_states: DisplayState,

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

  /// Close warnings [default: true]
  #[arg(long, default_value_t = false)]
  pub show_warnings: bool,

  /// rustc arguments
  rustc_args: Vec<String>,
}

impl Cli {
  pub fn from_rustc() -> Self {
    let mut cli = Cli::parse();
    cli.rustc_args.push(
        std::env::current_exe().expect("").to_str().unwrap().to_string()
    );
    cli.rustc_args.push(cli.file.to_string());
    cli.rustc_args.push("-Copt-level=1".to_string());
    // Reorder basic blocks to reverse post-order
    cli.rustc_args.push("-Zmir-enable-passes=+ReorderBasicBlocks".to_string());
    if !cli.show_warnings {
      cli.rustc_args.push("-Awarnings".to_string());
    }
    cli
  }

  pub fn from_cargo() -> Self {
    let mut rustc_args = std::env::args().into_iter().collect::<Vec<_>>();
    // TODO: set toolchain version
    rustc_args.remove(1);
    rustc_args.push("-Copt-level=1".to_string());
    rustc_args.push("-Zmir-enable-passes=+ReorderBasicBlocks".to_string());
    let mirv_flags =
      match std::env::var("MIRV_FLAGS") {
        Ok(flags) => flags,
        _ => NString::EMPTY.to_string(),
      };
    let mut mirv_args = vec!["."];
    mirv_args.append(&mut mirv_flags.split_whitespace().collect::<Vec<_>>());
    let mut cli = Cli::parse_from(mirv_args);
    cli.rustc_args = rustc_args;
    if !cli.show_warnings { cli.rustc_args.push("-Awarnings".to_string()); }
    cli
  }

  pub fn rustc_args(&self) -> Vec<String> {
    self.rustc_args.clone()
  }

  pub fn enable_display_state_bb(&self) -> bool {
    self.show_states == DisplayState::BB ||
      self.show_states == DisplayState::All
  }

  pub fn enable_display_state_statement(&self) -> bool {
    self.show_states == DisplayState::Statement ||
      self.show_states == DisplayState::All
  }

  pub fn enable_display_state_terminator(&self) -> bool {
    self.show_states == DisplayState::Terminator ||
      self.show_states == DisplayState::All
  }
}