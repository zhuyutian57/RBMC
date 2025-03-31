# MIR-V

**MIR-V** is a Bounded Model Checker for Rust based on [Stable MIR](https://github.com/rust-lang/project-stable-mir). It is still under development. We aim to develop a verifier for the memory safety problem in Rust.

## Installation

We recommend using `x.py` to build and install the binary tools. Alternatively, you can follow the standard installation instructions.

## Binary

We provide two binary tools, `mirv` and `cargo-mirv`. `mirv` is a wrapper of Rust. It will handle the commands and run the compiler. After compiling, `mirv` will start verifying the `.rs` by running BMC as a callback function of rustc. More details are shown by `-h`. `cargo-mirv` is used for a project. `cargo-mirv` will build the project by using `mirv` as the compiler. It is still under development.

## Rustc

`MIR-V` relies on `nightly-2025-03-02` rustc. The library of the nightly toolchain should be set before using our tool. We recommend using a temporary terminal and exporting the library by
```sh
export LD_LIBRARY_PATH=$(rustc --print sysroot)/lib:$LD_LIBRARY_PATH
```
in the root of `MIR-V`. 
