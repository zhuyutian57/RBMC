#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
#[macro_use]
extern crate rustc_smir;
extern crate stable_mir;

use rustc_smir::{run, rustc_internal};
use stable_mir::CompilerError;

use std::ops::ControlFlow;
use std::process::ExitCode;

use mir_v::bmc::bmc::Bmc;
use mir_v::config::cli::{self, Cli};
use mir_v::config::config::Config;

fn main() -> ExitCode {
    let cli = Cli::new();
    match run!(cli.rustc_args(), || mirv_bmc(cli)) {
        Ok(_) | Err(CompilerError::Skipped) | Err(CompilerError::Interrupted(_)) => {
            ExitCode::SUCCESS
        }
        _ => ExitCode::FAILURE,
    }
}

fn mirv_bmc(cli: Cli) -> ControlFlow<()> {
    // Verify when the current crate is variable.
    let local_crate = stable_mir::local_crate().name;
    if !cli.cur_crate().is_empty()
        || matches!(std::env::var(cli::MIRV_CRATE), Ok(x) if local_crate == x)
    {
        Bmc::new(&Config::new(cli)).do_bmc();
    }

    ControlFlow::Continue(())
}
