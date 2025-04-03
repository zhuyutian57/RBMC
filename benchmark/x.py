#!/usr/bin/env python3

# Script for experiment on benchmark

OUT_DIR = "./output/"
MIRV_OUTPUT = OUT_DIR + "mirv/"
KANI_OUTPUT = OUT_DIR + "kani/"

loop_bound = {
  "lockfree-3-0": 12,
  "lockfree-3-1": 7,
  "lockfree-3-2": 7,
  "lockfree-3-3": 7,
  "test-0232-1": 1,
  "test-0232-2": 10,
  "test-0232-3": 3,
}

def run_on_single_file(cmd, env, smt_strategy):
  # Save log in output
  crate = os.path.splitext(os.path.basename(cmd[1]))[0]
  out = ""
  log_file = ""
  extra_args = []
  if cmd[0] == "mirv":
    # set bound
    if crate in loop_bound:
      extra_args.append("--unwind")
      extra_args.append(str(loop_bound[crate]))
    out = MIRV_OUTPUT
    log_file = out + f"{crate}-{smt_strategy}.log"
  else:
    # set bound
    if crate in loop_bound:
      extra_args.append("--unwind")
      extra_args.append(str(loop_bound[crate]))
    out = KANI_OUTPUT
    log_file = out + f"{crate}.log"
  extra_args += [">", log_file]
  final_cmd = cmd + extra_args
  print("Go " + " ".join(final_cmd))
  if env == None: os.system(" ".join(final_cmd))
  else: os.system(env + " && " + " ".join(final_cmd))

def mirv(file):
  assert(os.path.exists(file))
  # set MIRV_LIBRARY_PATH
  mirv_lib = os.path.join(os.path.curdir, "../target/release/libmirv.rlib")
  os.environ["MIRV_LIBRARY_PATH"] = str(os.path.abspath(mirv_lib))
  # set RUSTUP_TOOLCHAIN
  env = "export LD_LIBRARY_PATH=$(rustc --print sysroot)/lib:$LD_LIBRARY_PATH"
  cmd = ["mirv", file]
  run_on_single_file(cmd, env, "forward")
  run_on_single_file(cmd, env, "once")
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
    "--memory-safety-checks",
    "--no-unwinding-checks",
    "-Z",
    "unstable-options",
    "--cbmc-args",
    "--memory-leak-check"
  ]
  run_on_single_file(cmd, None, None)

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

def analysis_mirv_result():
  assert(os.path.exists(MIRV_OUTPUT))
  # crate: (VCs, assertions, bugs, forward-time, once-time)
  results = {}
  for logfile in os.listdir(f"{MIRV_OUTPUT}"):
    if logfile.endswith("-forward.log"):
      res = [0, 0, set(), "", ""]
      with open(os.path.join(MIRV_OUTPUT, logfile)) as log:
        for line in log.readlines():
          if line.startswith("Generating"):
            res[0] = int(line.split(' ')[1])
            res[1] = int(line.split(' ')[4])
          if line.startswith("Verification time"):
            res[3] = line.split(" ")[2].strip("\n")
      once_logfile = logfile.replace("-forward.log", "-once.log")
      with open(os.path.join(MIRV_OUTPUT, once_logfile)) as log:
        for line in log.readlines():
          if "dereference failure" in line or \
             "index out of bounds" in line:
            res[2].add("ID")
          if "dealloc failure" in line or "drop failure" in line:
            res[2].add("IF")
          if "memory leak" in line:
            res[2].add("ML")
          if line.startswith("Verification time"):
            res[4] = line.split(" ")[2].strip("\n")
      crate = logfile.replace("-forward.log", ".rs")
      results[crate] = res
  for crate in sorted(results):
    res = [crate] + results[crate]
    res[3] = "/".join(res[3])
    print("{:<20} {:<10} {:<5} {:<5} {:<15} {:<15}".format(*res))
  
def analysis_kani_result():
  assert(os.path.exists(KANI_OUTPUT))
  # crate: (VCs, assertions, bugs, forward-time, once-time)
  results = {}
  for logfile in os.listdir(f"{KANI_OUTPUT}"):
    if not logfile.endswith(".log"): continue
    res = [0, 0, set(), ""]
    with open(os.path.join(KANI_OUTPUT, logfile)) as log:
      for line in log.readlines():
        if line.startswith("Generated"):
          res[0] = int(line.split(" ")[1])
        if line.startswith("Check"):
          res[1] += 1
        if line.startswith("Failed Checks"):
          if "dereference failure" in line or \
             "index out of bounds" in line or \
            "Offset result and original pointer must point to the same allocation" in line:
            res[2].add("ID")
          if "rust_dealloc" in line or \
            "double free" in line or \
            "free argument must be NULL or valid pointer" in line:
            res[2].add("IF")        
          if "dynamically allocated memory never freed" in line:
            res[2].add("ML")
        if line.startswith("Verification Time"):
          res[3] = line.split(" ")[2].strip("\n")
    crate = logfile.replace(".log", ".rs")
    results[crate] = res
  for crate in sorted(results):
    res = [crate] + results[crate]
    res[3] = "/".join(res[3])
    print("{:<20} {:<10} {:<5} {:<10} {:<15}".format(*res))

if __name__ == "__main__":
  import argparse
  import os

  parser = argparse.ArgumentParser()
  parser.add_argument(
    "--kani",
    action="store_true",
    help="Using kani as tool")
  parser.add_argument(
    "--analysis",
    action="store_true",
    help="Analisys result from output")

  args, dir = parser.parse_known_args()
  if not args.analysis:
    assert(len(dir) == 1)
    run_expriment(dir[0], "mirv" if not args.kani else "kani")
  else:
    if not args.kani:
      analysis_mirv_result()
    else:
      analysis_kani_result()