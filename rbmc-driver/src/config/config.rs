use std::cell::RefCell;

use stable_mir::target::MachineInfo;

use super::cli::*;
use crate::expr::context::*;
use crate::program::program::*;
use crate::solvers::context::SolverCtx;

pub struct Config {
    pub(crate) cli: Cli,
    pub(crate) machine_info: MachineInfo,
    pub(crate) program: Program,
    pub(crate) expr_ctx: ExprCtx,
    pub(crate) solver_config: SolverCtx,
}

impl Config {
    pub fn new(cli: Cli) -> Self {
        // Machine info
        let machine_info = MachineInfo::target();

        // Get stable mir
        let program = Program::new(stable_mir::local_crate(), cli.entry_function);

        // Context for managing Expr
        let expr_ctx = ExprCtx::new(RefCell::new(Context::new()));

        // Initilized solver
        let solver_config = SolverCtx::new(&cli);

        Config { cli, machine_info, program, expr_ctx, solver_config }
    }
}
