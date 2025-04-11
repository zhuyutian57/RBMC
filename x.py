#!/usr/bin/env python3

# Script for debug

def rbmc(file, mirv_args):
  assert(os.path.exists(file))
  # set MIRV_LIBRARY_PATH
  mirv_lib = os.path.join(os.path.curdir, "./target/debug/librbmc.rlib")
  os.environ["MIRV_LIBRARY_PATH"] = str(os.path.abspath(mirv_lib))
  cmd = ["cargo", "run", "--bin", "rbmc", file] + mirv_args
  os.system(" ".join(cmd))
  crate = os.path.splitext(os.path.basename(file))[0]
  if os.path.exists(crate): os.system(f'rm {crate}')

if __name__ == "__main__":
  import argparse
  import os
  import sys

  parser = argparse.ArgumentParser()
  parser.add_argument(
    "--build",
    action="store_true",
    help="Run `cargo build --all`")
  parser.add_argument(
    "--clean",
    action="store_true",
    help="Run `cargo clean`")
  parser.add_argument(
    "-f", "--file",
    type=str,
    help="Run `cargo run --bin rbmc *.rs`. The generated binary file will be deleted")
  parser.add_argument(
    "--install",
    action="store_true",
    help="Run `cargo install --path .`")
  parser.add_argument(
    "--uninstall",
    action="store_true",
    help="Run `cargo uinsntall rbmc`")

  args, mirv_args = parser.parse_known_args()

  if args.build:
    os.system("cargo build --all")
  elif args.clean:
    os.system("cargo clean")
  elif args.file is not None:
    rbmc(args.file, mirv_args)
  elif args.install:
    os.system("cd ./library && cargo build --release")
    os.system("cargo install --path .")
  elif args.uninstall:
    os.system("cargo uninstall rbmc")
  else:
    parser.print_help(sys.stdout)
