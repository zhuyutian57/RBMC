# RBMC

**RBMC** is a Bounded Model Checker for Rust based on [Stable MIR](https://github.com/rust-lang/project-stable-mir). It is still under development. We aim to develop a verifier for the memory safety in Rust.

## Installation

We recommend using `x.py` to build and install the binary tools. Alternatively, you can follow installation instructions in `.cargo/config.toml`.

## Binary

We provide two binary tools, `rbmc` and `cargo-rbmc`.
- `rbmc` is a wrapper of `bmc-driver`. It aims to fix the environment for running `bmc-driver`.
- `cargo-rbmc` is used for a project. `cargo-rbmc` will build the project by using `rbmc` as the compiler. It is still under development.

Moreover, `bmc-driver` is a wrapper of `rustc`. Our BMC algorithm is implemented as a callback function of `rustc`.

## MIRV
The older version is [MIRV](https://github.com/zhuyutian57/RBMC/tree/85bb1e0be607d49069a385d9ff52ba51b452668a)
