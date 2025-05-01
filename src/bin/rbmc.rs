use std::process::Command;

/// Set flags and run rbmc-driver
fn main() {
    rust_bmc::setup_toolchain();
    let status = Command::new("rbmc-driver")
        .env("PATH", rust_bmc::path())
        .env("RUSTC_ARGS", rust_bmc::rustc_args())
        .args(rust_bmc::rbmc_args())
        .status()
        .expect("Fail to run RBMC");
    assert!(status.success());
}
