use std::path::{Path, PathBuf};

// This `lib` is a wrapper for running `rbmc-driver`.
//
// For running `rbmc-driver`, we must do some works to retrieve correct MIR,
// including setting toolchain and setting `RUSTCFLAGS`.
// 
// Moreover, to retrieve the MIR of `std`, we must link the compiled `std`
// libraries. That means we should reset the `--sysroot` via `rustc` argument.

const VERSION: &str = std::env!("CARGO_PKG_VERSION");

/// Export bin folder in path
pub fn path() -> String {
    let rbmc_bin = &[rbmc_bin()];
    match std::env::var_os("PATH") {
        Some(path) => {
            let orig = std::env::split_paths(&path);
            std::env::join_paths(rbmc_bin.iter().cloned().chain(orig))
        }
        _   => std::env::join_paths(rbmc_bin),
    }.unwrap().to_str().unwrap().into()
}

/// The directory where `RBMC` is installed.
/// Default directory is `$HOME/.rbmc/rbmc-<VERSION>`.
pub fn rbmc_home() -> PathBuf {
  match std::env::var("RBMC_HOME") {
    Ok(path) => PathBuf::from(path),
    _ => home::home_dir().expect("Not home directory")
        .join(".rbmc").join(format!("rbmc-{VERSION}")),
  }
}

pub fn rbmc_bin() -> PathBuf {
    rbmc_home().join("bin")
}

pub fn rbmc_lib() -> PathBuf {
    rbmc_home().join("lib")
}

pub fn rbmc_args() -> Vec<String> {
    std::env::args().skip(1).into_iter().collect()
}

pub fn rustc_args() -> String {
    let rbmc_home = rbmc_home();
    let librbmc = rbmc_home.join("lib").join("librbmc.rlib");
    [   // Set sysroot
        "--sysroot", rbmc_home.to_str().unwrap(),
        // Link the compiled libraries
        "-L", rbmc_lib().to_str().unwrap(),
        // Link rbmc lib for non-deterministic variable
        "--extern", format!("rbmc={librbmc:?}").as_str(),
        // Other arguement for compiling
        "-Awarnings",
        "-Copt-level=1",
        "-Zalways-encode-mir",
        "-Zmir-enable-passes=+ReorderBasicBlocks"
    ].join(" ")
}