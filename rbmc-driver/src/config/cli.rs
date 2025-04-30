use clap::*;

use crate::symbol::nstring::NString;

pub const RBMC_CRATE: &str = "RBMC_CRATE";
pub const RBMC_FLAGS: &str = "RBMC_FLAGS";

#[derive(clap::ValueEnum, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ProgramInfo {
    #[default]
    Local,
    All,
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

#[derive(clap::ValueEnum, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SmtStrategy {
    #[default]
    Forward,
    Once,
}

#[derive(Parser, Debug, Default)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Source file '.rs'
    #[arg(default_value_t = NString::EMPTY)]
    pub file: NString,

    /// Entry function
    #[arg(long, default_value_t = NString::from("main"))]
    pub entry_function: NString,

    /// Loop bound. '0' indicates unbounded
    #[arg(long, default_value_t = 0)]
    pub unwind: usize,

    /// Show program
    #[arg(long, default_value_t = false)]
    pub show_program: bool,

    /// Show program and terminate
    #[arg(long, default_value_t = false)]
    pub program_only: bool,

    /// Set program info for display.
    /// `Local` for displaying functions in current crate and
    /// `All` for displaying all reachable non-builtin functions.
    #[clap(value_enum)]
    #[arg(long, default_value_t = ProgramInfo::Local)]
    pub program_info: ProgramInfo,

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
    #[arg(long, default_value_t = SmtStrategy::Once)]
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
        match std::env::var(RBMC_CRATE) {
            // From `cargo-rbmc` or `cargo rbmc`
            Ok(_crate) => {
                let mirv_args = match std::env::var(RBMC_FLAGS) {
                    Ok(flags) => {
                        flags.split_whitespace().map(|arg| arg.to_string()).collect::<Vec<_>>()
                    }
                    _ => vec![],
                };
                Cli::parse_from(mirv_args)
            }
            // From `rbmc *.rs`
            Err(_) => Cli::parse(),
        }
    }

    pub fn rustc_args(&self) -> Vec<String> {
        let mut args = vec![std::env::current_exe().unwrap().to_str().unwrap().into()];
        // TODO: fix cargo rbmc
        args.push(self.file.to_string());
        let extra_args = std::env::var("RUSTC_ARGS")
            .unwrap()
            .split(' ')
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        args.extend(extra_args);
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
