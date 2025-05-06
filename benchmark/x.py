#!/usr/bin/env python3

# Script for experiment on benchmark

OUT_DIR = "./output/"
RBMC_OUTPUT = OUT_DIR + "rbmc/"
KANI_OUTPUT = OUT_DIR + "kani/"
ESBMC_OUTPUT = OUT_DIR + "esbmc/"

loop_bound = {
  "lockfree-3-0": 12,
  "lockfree-3-1": 7,
  "lockfree-3-2": 7,
  "lockfree-3-3": 11,
  "test-0232-1": 1,
  "test-0232-2": 10,
  "test-0232-3": 3,

  # C file
  "lockfree-3.0": 12,
  "lockfree-3.1": 7,
  "lockfree-3.2": 7,
  "lockfree-3.3": 11,
}

def run_on_single_file(cmd, smt_strategy):
  # Save log in output
  crate = os.path.splitext(os.path.basename(cmd[1]))[0]
  out = ""
  log_file = ""
  extra_args = []
  
  # set bound
  if crate in loop_bound:
    extra_args.append("--unwind")
    extra_args.append(str(loop_bound[crate]))

  if cmd[0] == "rbmc":
    out = RBMC_OUTPUT
    log_file = out + f"{crate}-{smt_strategy}.log"
  else:
    out = KANI_OUTPUT if "kani" in cmd[0] else ESBMC_OUTPUT
    log_file = out + f"{crate}.log"
  extra_args += [">", log_file, "2>&1"]
  final_cmd = cmd + extra_args
  print("Go " + " ".join(final_cmd))
  os.system(" ".join(final_cmd))

def rbmc(file):
  assert(os.path.exists(file))
  cmd = ["rbmc", file]
  run_on_single_file(cmd, "forward")
  run_on_single_file(cmd, "once")
  crate = os.path.splitext(os.path.basename(file))[0]
  if os.path.exists(crate): os.system(f'rm {crate}')

def kani(file):
  assert(os.path.exists(file))
  code = []
  with open(file, "r") as crate:
    for line in crate.readlines():
      if line.startswith("fn main() {"):
        code.append("#[kani::proof]\n")
      if "extern crate rbmc;" in line: continue
      code.append(line.replace("rbmc::nondet", "kani::any"))
  # crate a tmp file
  tmp_file = os.path.join(KANI_OUTPUT, os.path.basename(file))
  with open(tmp_file, "w") as crate: crate.write("".join(code))

  # close warnings
  os.environ["RUSTFLAGS"] = "-Awarnings -Copt-level=1"

  cmd = [
    "kani",
    tmp_file,
    "--no-default-checks",
    "--no-unwinding-checks",
    "--no-overflow-checks",
    "--no-assertion-reach-checks",
    "--memory-safety-checks",
    "-Z",
    "unstable-options",
    "--cbmc-args",
    "--memory-leak-check",
    "--no-malloc-may-fail",
  ]
  run_on_single_file(cmd, None)

def esbmc(file):
  assert(os.path.exists(file) and file.endswith(".c"))

  # ESBMC path
  esbmc = "../../esbmc/build/src/esbmc/esbmc"

  cmd = [
    esbmc,
    file,
    "--force-malloc-success",
    "--memory-leak-check",
    "--no-unwinding-assertions"
  ]
  run_on_single_file(cmd, None)

def run_expriment(dir, tool):
  print(f"Run experiment in {dir} with {tool.upper()}")

  if not os.path.exists(OUT_DIR): os.mkdir(OUT_DIR)
  if tool == "rbmc" and not os.path.exists(RBMC_OUTPUT): os.mkdir(RBMC_OUTPUT)
  if tool == "kani" and not os.path.exists(KANI_OUTPUT): os.mkdir(KANI_OUTPUT)
  if tool == "esbmc" and not os.path.exists(ESBMC_OUTPUT): os.mkdir(ESBMC_OUTPUT)

  crates = []
  for crate in os.listdir(f"{dir}"):
    if crate.endswith(".rs"):
      crates.append(crate)
    elif crate.endswith(".c") and tool == "esbmc":
      crates.append(crate)
  crates.sort()

  for crate in crates:
    file = os.path.join(dir, crate)
    if tool == "rbmc" : rbmc(file)
    elif tool == "kani": kani(file)
    else: esbmc(file)

def format_res(results):
  for crate in sorted(results):
    res = [crate] + results[crate]
    if len(res[3]) == 0: res[3] = "-"
    else: res[3] = "/".join(res[3])
    if len(res) == 5 :
      print("{:<20} {:<10} {:<7} {:<7} {:.5f}s".format(*res))
    else:
      print("{:<20} {:<10} {:<7} {:<7} {:.5f}s  {:.5f}s".format(*res))

def analysis_rbmc_result():
  assert(os.path.exists(RBMC_OUTPUT))
  # crate: (VCs, assertions, bugs, forward-time, once-time)
  results = {}
  for logfile in os.listdir(f"{RBMC_OUTPUT}"):
    if logfile.endswith("-forward.log"):
      res = [0, 0, set(), 0.0, 0.0]
      with open(os.path.join(RBMC_OUTPUT, logfile)) as log:
        for line in log.readlines():
          if line.startswith("Generating"):
            res[0] = int(line.split(' ')[1])
            res[1] = int(line.split(' ')[4])
          if line.startswith("Verification time"):
            res[3] = float(line.split(" ")[2].strip("\n").strip('s'))
      once_logfile = logfile.replace("-forward.log", "-once.log")
      with open(os.path.join(RBMC_OUTPUT, once_logfile)) as log:
        for line in log.readlines():
          if "dereference failure" in line or \
             "index out of bounds" in line:
            res[2].add("ID")
          if "dealloc failure" in line or "drop failure" in line:
            res[2].add("IF")
          if "memory leak" in line:
            res[2].add("ML")
          if line.startswith("Verification time"):
            res[4] = float(line.split(" ")[2].strip("\n").strip('s'))
      crate = logfile.replace("-forward.log", ".rs")
      results[crate] = res
  format_res(results)
  
def analysis_kani_result():
  assert(os.path.exists(KANI_OUTPUT))
  # crate: (VCs, assertions, bugs, time)
  results = {}
  for logfile in os.listdir(f"{KANI_OUTPUT}"):
    if not logfile.endswith(".log"): continue
    res = [0, 0, set(), 0.0]
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
          res[3] = float(line.split(" ")[2].strip("\n").strip('s'))
    crate = logfile.replace(".log", ".rs")
    results[crate] = res
  format_res(results)

def analysis_esbmc_result():
  assert(os.path.exists(ESBMC_OUTPUT))
  # crate: (VCs, assertions, bugs, time)
  results = {}
  for logfile in os.listdir(f"{ESBMC_OUTPUT}"):
    if not logfile.endswith(".log"): continue
    res = [0, 0, set(), 0.0]
    with open(os.path.join(ESBMC_OUTPUT, logfile)) as log:
      for line in log.readlines():
        if line.startswith("Symex completed in:"):
          res[0] = int(line.split(" ")[4][1:])
        if line.startswith("Generated"):
          res[1] = int(line.split(" ")[3])

        if "array bounds violated" in line:
          res[2].add("ID")
        if "Operand of free must have zero pointer offset" in line \
          or "dereference failure: invalidated dynamic object freed" in line \
          or "invalid pointer freed" in line:
          res[2].add("IF")        
        if "forgotten memory:" in line:
          res[2].add("ML")
        
        # if line.startswith("GOTO program creation time:"):
        #   res[3] += float(line.split(" ")[-1].strip('\n').strip('s'))
        # if line.startswith("GOTO program processing time:"):
        #   res[3] += float(line.split(" ")[-1].strip('\n').strip('s'))
        if line.startswith("Symex completed in:"):
          res[3] += float(line.split(" ")[3].strip('s'))
        if line.startswith("Slicing time:"):
          res[3] += float(line.split(" ")[2].strip('s'))
        if line.startswith("Encoding to solver time:"):
          res[3] += float(line.split(" ")[-1].strip('\n').strip('s'))
        if line.startswith("Runtime decision procedure:"):
          res[3] += float(line.split(" ")[-1].strip('\n').strip('s'))

        if line.startswith("Assertions:"):
          res[1] = int(line.split(" ")[1].strip('\n'))

    crate = logfile.replace(".log", ".c")
    results[crate] = res
  format_res(results)

if __name__ == "__main__":
  import argparse
  import os
  import subprocess

  parser = argparse.ArgumentParser()
  parser.add_argument(
    "--kani",
    action="store_true",
    help="Using kani as tool")
  parser.add_argument(
    "--esbmc",
    action="store_true",
    help="Using esbmc as tool")
  parser.add_argument(
    "--analysis",
    action="store_true",
    help="Analisys result from output")

  args, dir = parser.parse_known_args()
  if not args.analysis:
    assert(len(dir) == 1)
    assert(args.kani + args.esbmc <= 1)
    if args.kani: run_expriment(dir[0], "kani")
    elif args.esbmc: run_expriment(dir[0], "esbmc")
    else: run_expriment(dir[0], "rbmc")
  else:
    if args.kani:
      analysis_kani_result()
    elif args.esbmc:
      analysis_esbmc_result()
    else:
      analysis_rbmc_result()