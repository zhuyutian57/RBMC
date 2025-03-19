
use std::process::Command;

use cargo_metadata::*;

const MIRV_MODE : &str = "cargo";

fn parse_mirv_flags() -> String {
  let mut args = std::env::args().into_iter().collect::<Vec<_>>();
  let idx =
    args
      .iter()
      .enumerate()
      .find_map(
        |(i, x)|
        if x == "--mirv-flags" { Some(i) } else { None }
      );
  match idx {
    Some(i) => args.split_off(i + 1).join(" "),
    None => "".to_string(),
  }
}

fn main() {
  let metadata_cmd = cargo_metadata::MetadataCommand::new();
  let metadata =
    match metadata_cmd.exec() {
      Ok(m) => m,
      _ => panic!("Fail to get the metadata of the current project"),
    };
  
  // We only check the current crate
  if let Some(root) = metadata.root_package() {
    println!("cargo mirv: {}", root.name);
    let mut cmd = Command::new("cargo");
    cmd
      // Tell mirv to retrieve rustc args diretly
      .env("MIRV_MODE", MIRV_MODE)
      // Set the crate being verified
      .env("MIRV_CRATE", root.name.as_str())
      // Mirv arguments
      .env("MIRV_FLAGS", parse_mirv_flags())
      // Wrap the rustc with mirv
      .env("RUSTC_WRAPPER", "mirv")
      // No need to compile the whole project
      .arg("check").arg("--bin").arg(root.name.as_str());
    
    let exit_status = cmd
      .spawn()
      .expect("could not run cargo")
      .wait()
      .expect("failed to wait for cargo");
  
    if !exit_status.success() {
      std::process::exit(exit_status.code().unwrap_or(-1))
    }
  } else {  
    panic!("Not support lib yet");
  }
}