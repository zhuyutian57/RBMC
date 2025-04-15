use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Set flags and run rbmc-driver
fn main() {
    let status =
        Command::new("rbmc-driver")
            .env("PATH", rust_bmc::path())
            .env("RUSTCFLAGS", rust_bmc::rustc_flags())
            .args(rust_bmc::rbmc_args())
            .status()
            .expect("Fail to run RBMC");
    assert!(status.success());
    remove_generated_binary();
}

fn remove_generated_binary() {
    let args = rust_bmc::rbmc_args();
    let cur = std::env::current_dir().unwrap();
    if let Some(s) = args.iter().find(|&x| x.ends_with(".rs")) {
        let file = PathBuf::from(s);
        let exec = cur.join(file.file_stem().unwrap());
        if exec.exists() {
            std::fs::remove_file(exec);
        }
    }
    
}