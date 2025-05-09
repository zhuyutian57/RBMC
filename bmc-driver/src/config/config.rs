use std::cell::RefCell;

use super::cli::*;
use crate::expr::context::*;
use crate::program::program::*;
use crate::solvers::context::SolverCtx;
use crate::symbol::nstring::NString;

pub struct Config {
    pub(crate) cli: Cli,
    pub(crate) program: Program,
    pub(crate) expr_ctx: ExprCtx,
    pub(crate) solver_config: SolverCtx,
}

impl Config {
    pub fn new(cli: Cli) -> Self {
        // Get stable mir
        let program = Program::new(stable_mir::local_crate(), cli.entry_function);

        // Context for managing Expr
        let expr_ctx = ExprCtx::new(RefCell::new(Context::new()));

        // Initilized solver
        let solver_config = SolverCtx::new(&cli);

        Config { cli, program, expr_ctx, solver_config }
    }

    pub fn enable_display_state(&self) -> bool {
        self.cli.show_state != DisplayState::None
    }

    pub fn enable_display_state_statement(&self) -> bool {
        self.cli.show_state == DisplayState::Statement || self.cli.show_state == DisplayState::All
    }

    pub fn enable_display_state_terminator(&self) -> bool {
        self.cli.show_state == DisplayState::Terminator || self.cli.show_state == DisplayState::All
    }

    pub fn enable_display_state_in_function(&self, name: NString) -> bool {
        self.program.is_local_function(name) || self.cli.show_std_state
    }
}
