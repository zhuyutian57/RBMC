#!/usr/bin/env python3

# Script for experiment on benchmark

OUT_DIR = "./output/"
MIRV_OUTPUT = OUT_DIR + "mirv/"
KANI_OUTPUT = OUT_DIR + "kani/"

def run_on_single_file(cmd, env):
  # Save log in output
  crate = os.path.splitext(os.path.basename(cmd[1]))[0]
  out = ""
  if cmd[0] == "mirv": out = MIRV_OUTPUT
  else: out = KANI_OUTPUT
  log_file = out + f"{crate}.log"
  cmd += [">", log_file]
  print("Go " + " ".join(cmd))
  if env == None: os.system(" ".join(cmd))
  else: os.system(env + " && " + " ".join(cmd))

def mirv(file):
  assert(os.path.exists(file))
  # set MIRV_LIBRARY_PATH
  mirv_lib = os.path.join(os.path.curdir, "../target/release/libmirv.rlib")
  os.environ["MIRV_LIBRARY_PATH"] = str(os.path.abspath(mirv_lib))
  # set RUSTUP_TOOLCHAIN
  env = "export LD_LIBRARY_PATH=$(rustc --print sysroot)/lib:$LD_LIBRARY_PATH"
  cmd = ["mirv", file]
  run_on_single_file(cmd, env)
  crate = os.path.splitext(os.path.basename(file))[0]
  if os.path.exists(crate): os.system(f'rm {crate}')

def kani(file):
  assert(os.path.exists(file))
  code = []
  with open(file, "r") as crate:
    for line in crate.readlines():
      if line.startswith("fn main() {"):
        code.append("#[kani::proof]\n")
      if "extern crate mirv;" in line: continue
      code.append(line.replace("mirv::nondet", "kani::any"))
  # crate a tmp file
  tmp_file = os.path.join(KANI_OUTPUT, os.path.basename(file))
  with open(tmp_file, "w") as crate: crate.write("".join(code))

  # close warnings
  os.environ["RUSTFLAGS"] = "-Awarnings"

  cmd = [
    "kani",
    tmp_file,
    "--no-default-checks",
    "--memory-safety-checks"
  ]
  run_on_single_file(cmd, None)

def run_expriment(dir, tool):
  print(f"Run experiment in {dir} with {tool.upper()}")

  if not os.path.exists(OUT_DIR): os.mkdir(OUT_DIR)
  if tool == "mirv" and not os.path.exists(MIRV_OUTPUT): os.mkdir(MIRV_OUTPUT)
  if tool == "kani" and not os.path.exists(KANI_OUTPUT): os.mkdir(KANI_OUTPUT)

  crates = []
  for crate in os.listdir(f"{dir}"):
    if crate.endswith(".rs"):
      crates.append(crate)
  crates.sort()

  for crate in crates:
    file = os.path.join(dir, crate)
    if tool == "mirv" : mirv(file)
    else: kani(file)

if __name__ == "__main__":
  import argparse
  import os

  parser = argparse.ArgumentParser()
  parser.add_argument(
    "--kani",
    action="store_true",
    help="Using kani as tool")

  args, dir = parser.parse_known_args()
  assert(len(dir) == 1)
  run_expriment(dir[0], "mirv" if not args.kani else "kani")