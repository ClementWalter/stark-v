# builder

Guest build orchestration.

## Overview

The `builder` crate provides a high-level API to compile Rust guest crates for
the `riscv32im-risc0-zkvm-elf` target. It automates the `cargo` invocation with
the correct toolchain, linker flags, and configuration for stark-v compatible
guests.

## Features

- Compiles guest crates using the `risc0` Rust toolchain
- Sets the text section start address at `0x0020_0800`
- Enables `stark_v_runtime` cfg flag for conditional compilation
- Parses `Cargo.toml` to locate the output ELF binary

## Usage

```rust
use std::path::Path;
use builder::build_guest;

let output = build_guest(Path::new("path/to/guest"))?;
println!("ELF built at: {}", output.elf_path.display());
```

## Build Configuration

| Setting    | Value                      |
| ---------- | -------------------------- |
| Target     | `riscv32im-risc0-zkvm-elf` |
| Toolchain  | `risc0`                    |
| Profile    | `release`                  |
| Text start | `0x0020_0800`              |

## Error Handling

The crate provides detailed error types via `BuilderError`:

- `MissingCargoToml` - Guest directory lacks a `Cargo.toml`
- `ReadFile` / `ParseToml` - I/O or TOML parsing failures
- `BuildGuest` / `GuestBuildFailed` - Cargo build invocation errors
- `ExpectedGuestElf` - Output ELF not found after build
