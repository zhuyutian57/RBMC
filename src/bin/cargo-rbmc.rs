#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_smir;
extern crate stable_mir;

use std::process::Command;

use rust_bmc::config::cli;

fn parse_mirv_flags() -> String {
    let mut args = std::env::args().into_iter().collect::<Vec<_>>();
    let idx =
        args.iter().enumerate().find_map(|(i, x)| if x == "--rbmc-args" { Some(i) } else { None });
    match idx {
        Some(i) => args.split_off(i).join(" "),
        None => "".to_string(),
    }
}

fn main() {
    let metadata_cmd = cargo_metadata::MetadataCommand::new();
    let metadata = match metadata_cmd.exec() {
        Ok(m) => m,
        _ => panic!("Fail to get the metadata of the current project"),
    };

    // We only check the current crate
    if let Some(root) = metadata.root_package() {
        for target in &root.targets {
            if target.is_test() {
                continue;
            }
            println!("Target: {}", target.name);
            let mut cmd = Command::new("cargo");
            cmd
                // Set the crate being verified
                .env(cli::RBMC_CRATE, target.name.as_str())
                // Mirv arguments
                .env(cli::RBMC_FLAGS, parse_mirv_flags())
                // Wrap the rustc with rbmc
                .env("RUSTC_WRAPPER", "rbmc")
                // No need to compile the whole project
                .arg("build");

            let exit_status =
                cmd.spawn().expect("could not run cargo").wait().expect("failed to wait for cargo");

            if !exit_status.success() {
                std::process::exit(exit_status.code().unwrap_or(-1))
            }
        }
    } else {
        panic!("Not support lib yet");
    }
}
