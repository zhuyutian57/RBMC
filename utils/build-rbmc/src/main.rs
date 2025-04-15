use std::path::{Path, PathBuf};
use std::process::Command;

use cargo_metadata::{Artifact, Message, TargetKind};

fn main() {
    if std::path::Path::exists(&build_root()) {
        std::fs::remove_dir_all(build_root()).unwrap();
    }
    build_bin();
    build_libs();
}

fn build_root() -> PathBuf {
    std::env!("RBMC_BUILD_ROOT").into()
}

fn build_target() -> &'static str {
    std::env!("TARGET")
}

fn build_bin() {
    let output_dir = build_root().join("bin");
    let args = [
        "--bins",
        "-Z",
        "unstable-options",
        "--artifact-dir",
        output_dir.to_str().unwrap()
    ];
    Command::new("cargo")
        .arg("build")
        .args(args)
        .status()
        .expect("Fail to build binaries");
}

fn build_libs() {
    let output_dir = build_root().join("lib");
    let rustc_flags = [
        "-Copt-level=1",
        "-Zalways-encode-mir",
        "-Zmir-enable-passes=+ReorderBasicBlocks"
    ];
    let extra_args = [
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
    let mut cmd = Command::new("cargo")
        .env("RUSTFLAGS", rustc_flags.join(" "))
        .arg("build")
        .args(extra_args)
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