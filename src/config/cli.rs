use clap::*;

use crate::symbol::nstring::NString;

pub const MIRV_CRATE: &str = "MIRV_CRATE";
pub const MIRV_LIBRARY_PATH: &str = "MIRV_LIBRARY_PATH";
pub const MIRV_FLAGS: &str = "MIRV_FLAGS";

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
}

impl Cli {
    pub fn new() -> Self {
        // Carefully, dno't print anything in this function.
        // The println! will communicate with rustc directly.
        match std::env::var(MIRV_CRATE) {
            // From `cargo-mirv` or `cargo mirv`
            Ok(_crate) => {
                let mirv_args = match std::env::var(MIRV_FLAGS) {
                    Ok(flags) => {
                        flags.split_whitespace().map(|arg| arg.to_string()).collect::<Vec<_>>()
                    }
                    _ => vec![],
                };
                Cli::parse_from(mirv_args)
            }
            // From `mirv *.rs`
            Err(_) => Cli::parse(),
        }
    }

    pub fn rustc_args(&self) -> Vec<String> {
        let mut args = match self.file.is_empty() {
            // From `cargo-mirv` or `cargo mirv`
            true => std::env::args().skip(1).into_iter().collect(),
            // From `mirv *.rs`
            false => vec![
                std::env::current_exe().unwrap().to_str().unwrap().to_string(),
                self.file.to_string(),
            ],
        };
        if !self.show_warnings {
            args.push("-Awarnings".to_string());
        }
        args.push("-Copt-level=1".to_string());
        args.push("-Zalways-encode-mir".to_string());
        args.push("-Zmir-enable-passes=+ReorderBasicBlocks".to_string());
        // Link libmirv.rlib
        if let Ok(l) = std::env::var(MIRV_LIBRARY_PATH) {
            args.push("--extern".into());
            args.push(format!("mirv={l}").to_string());
        }
        args
    }

    pub fn cur_crate(&self) -> NString {
        if self.file.is_empty() {
            NString::EMPTY
        } else {
            let n = self.file.len();
            self.file.sub_str(0, n - 3)
        }
    }

    pub fn enable_display_state_bb(&self) -> bool {
        self.show_states == DisplayState::BB || self.show_states == DisplayState::All
    }

    pub fn enable_display_state_statement(&self) -> bool {
        self.show_states == DisplayState::Statement || self.show_states == DisplayState::All
    }

    pub fn enable_display_state_terminator(&self) -> bool {
        self.show_states == DisplayState::Terminator || self.show_states == DisplayState::All
    }
}
