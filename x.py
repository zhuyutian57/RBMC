#!/usr/bin/env python3

# Script for debug

def rbmc(file, rbmc_args):
  assert(os.path.exists(file))
  # set RBMC_LIBRARY_PATH
  rbmc_home = os.path.join(os.path.curdir, "target/rbmc")
  os.environ["RBMC_HOME"] = str(os.path.abspath(rbmc_home))
  cmd = ["cargo", "run", "--bin", "rbmc", "--", file] + rbmc_args
  os.system(" ".join(cmd))

if __name__ == "__main__":
  import argparse
  import os
  import sys

  parser = argparse.ArgumentParser()
  parser.add_argument(
    "--build",
    action="store_true",
    help="Run `cargo build-rbmc`")
  parser.add_argument(
    "--clean",
    action="store_true",
    help="Run `cargo clean`")
  parser.add_argument(
    "--install",
    action="store_true",
    help="Run `cargo install-rbmc`")
  parser.add_argument(
    "--uninstall",
    action="store_true",
    help="Run `cargo uinsntall-rbmc`")
  parser.add_argument(
    "-r", "--file",
    type=str,
    help="Run `cargo run --bin rbmc *.rs -- <RBMC_ARGS>`")

  args, rbmc_args = parser.parse_known_args()

  if args.build:
    os.system("cargo build-rbmc")
  elif args.clean:
    os.system("cargo clean")
  elif args.install:
    os.system("cargo install-rbmc")
  elif args.uninstall:
    os.system("cargo uninstall-rbmc")
  elif args.file is not None:
    os.system("cargo build-rbmc")
    rbmc(args.file, rbmc_args)
  else:
    parser.print_help(sys.stdout)
