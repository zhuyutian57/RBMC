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

use rust_bmc::bmc::bmc::Bmc;
use rust_bmc::config::cli::{self, Cli};
use rust_bmc::config::config::Config;

fn main() -> ExitCode {
    let cli = Cli::new();
    match run!(cli.rustc_args(), || start_bmc(cli)) {
        Ok(_) | Err(CompilerError::Skipped) | Err(CompilerError::Interrupted(_)) => {
            ExitCode::SUCCESS
        }
        _ => ExitCode::FAILURE,
    }
}

fn start_bmc(cli: Cli) -> ControlFlow<()> {
    // Verify when the current crate is variable.
    let local_crate = stable_mir::local_crate().name;
    if !cli.cur_crate().is_empty()
        || matches!(std::env::var(cli::RBMC_CRATE), Ok(x) if local_crate == x)
    {
        Bmc::new(&Config::new(cli)).do_bmc();
    }

    ControlFlow::Continue(())
}
