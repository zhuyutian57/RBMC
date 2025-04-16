use std::path::{Path, PathBuf};
use std::process::Command;

use cargo_metadata::{Artifact, Message, TargetKind};
use clap::*;

#[derive(Parser, Debug, Default)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Install RBMC
    #[arg(long, default_value_t = false)]
    install: bool,
}

static mut CLI: Cli = Cli { install: false };

#[inline]
fn parse_args() { unsafe {  CLI = Cli::parse(); } }

#[inline]
fn is_install() -> bool { unsafe  { CLI.install } }

const VERSION: &str = std::env!("CARGO_PKG_VERSION");

fn main() {
    parse_args();
    if std::path::Path::exists(&build_root()) {
        std::fs::remove_dir_all(build_root()).unwrap();
    }
    build_bin();
    build_libs();
    if is_install() {
        install();
    }
}

#[inline]
fn build_root() -> PathBuf {
    std::env!("RBMC_BUILD_ROOT").into()
}

#[inline]
fn build_target() -> &'static str {
    std::env!("TARGET")
}

fn build_bin() {
    let output_dir = build_root().join("bin");
    let mut args = vec![
        "--bins",
        "-Z",
        "unstable-options",
        "--artifact-dir",
        output_dir.to_str().unwrap()
    ];
    if is_install() { args.push("--release"); }
    Command::new("cargo")
        .arg("build")
        .args(args)
        .status()
        .expect("Fail to build binaries");
}

fn build_libs() {
    let output_dir = build_root().join("lib");
    let rustc_args = [
        "-Copt-level=1",
        "-Zalways-encode-mir",
        "-Zmir-enable-passes=+ReorderBasicBlocks"
    ];
    let mut args = vec![
        "-p",
        "rbmc",
        "-Z",
        "build-std=std",
        "-Z",
        "unstable-options",
        // The artifact `library` is copied into `target/rbmc/lib`
        "--artifact-dir",
        output_dir.to_str().unwrap(),
        "--message-format",
        "json-render-diagnostics"
    ];
    if is_install() { args.push("--release"); }
    let mut cmd = Command::new("cargo")
        .env("RUSTFLAGS", rustc_args.join(" "))
        .arg("build")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Fail to build libraries");
    
    // Retrieve artifacts
    let reader = std::io::BufReader::new(cmd.stdout.take().unwrap());
    let artifacts = Message::parse_stream(reader)
        .filter_map(
            |message|
            match message.unwrap() {
                Message::CompilerMessage(msg) => {
                    println!("{:?}", msg);
                    None
                },
                Message::CompilerArtifact(artifact) => Some(artifact),
                 _ => None,
            }
        )
        .collect::<Vec<_>>();
    if !cmd.wait().expect("Couldn't get exit status").success() {
        panic!("Compile libraries fails");
    }
    // Copy std libraries
    copy_std_lib(&artifacts);
}

fn is_rust_lib(artifact: &Artifact) -> bool {
    artifact.target.kind.iter().any(|kind| match kind {
        TargetKind::Lib | TargetKind::RLib | TargetKind::ProcMacro => true,
        TargetKind::Bin
        | TargetKind::DyLib
        | TargetKind::CDyLib
        | TargetKind::StaticLib
        | TargetKind::CustomBuild => false,
        _ => unreachable!("Unknown crate type {kind}"),
    })
}

fn is_rbmc_lib(artifact: &Artifact) -> bool {
    is_rust_lib(artifact) && artifact.target.src_path.starts_with(env!("REPO_ROOT"))
}

fn is_std_lib(artifact: &Artifact) -> bool {
    is_rust_lib(artifact) && !is_rbmc_lib(artifact)
}

fn cp(src: &Path, dst: &Path) {
    assert!(std::path::Path::is_dir(dst));
    let dst = dst.join(src.file_name().unwrap());
    std::fs::copy(src, dst).expect("Copy fail");
}

fn copy_std_lib(artifacts: &[Artifact]) {
    let std_path = build_root()
        .join("lib")
        .join("rustlib")
        .join(build_target())
        .join("lib");
    std::fs::create_dir_all(std_path.clone())
        .expect(&format!("Fail to create {std_path:?}"));
    artifacts.iter()
        .filter(|&artifact| is_std_lib(artifact))
        .for_each(
            |artifact|
            artifact
                .filenames
                .iter()
                .filter(
                    |&path|
                    path.extension() == Some("rlib") || path.extension() == Some(".so")
                )
                .for_each(
                    |lib|
                    cp(lib.as_std_path(), std_path.as_path())
                )
        );
}

fn install() {
    let build_root = build_root(); 
    assert!(build_root.exists());
    let rbmc_home = home::home_dir().unwrap()
        .join(".rbmc")
        .join(format!("rbmc-{VERSION}"));
    if rbmc_home.exists() {
        std::fs::remove_dir_all(rbmc_home.clone())
            .expect(&format!("Fail to create {rbmc_home:?}"));
    }
    std::fs::create_dir_all(rbmc_home.clone())
            .expect(&format!("Fail to create {rbmc_home:?}"));
    let status = Command::new("cp")
        .arg("-rf")
        .arg(build_root.join("."))
        .arg(rbmc_home)
        .status()
        .expect("Fail to copy");
    assert!(status.success());
    
    // Install binaries
    Command::new("cargo")
        .arg("install")
        .arg("--path")
        .arg(".")
        .status()
        .expect("Fail to install binaries");
}