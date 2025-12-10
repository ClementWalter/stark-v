# stark-v

A general purpose zkVM building on top of
[Stwo](https://github.com/starkware-libs/stwo).

## Overview

stark-v is a modular toolkit for building, transpiling, and proving RISC-V guest
programs targeting a zkVM. The workspace provides:

- A CLI to compile guest crates and inspect transpilation results
- A runtime library for writing `no_std` guest programs
- A transpiler that converts RV32IM ELF binaries to VM instructions
- A prover that generates a STARK proof from an execution trace

## Quick Start

Build the sample guest:

```bash
cargo run -- build --guest-path guests/playground
```

This compiles the `guests/playground` package and prints a summary containing
the ELF path, instruction count, and initialized memory size.

## Workspace Structure

```text
stark-v/
├── crates/
│   ├── builder/      # Guest build orchestration
│   ├── cli/          # Command-line interface (stark-v binary)
│   ├── prover/       # Proving infrastructure (WIP)
│   ├── stark-v/      # Guest runtime library
│   └── transpiler/   # ELF to VmExe transpiler
└── guests/
    └── playground/   # Sample guest program
```

## Crates

### [builder](crates/builder/)

Compiles Rust guest crates for the `riscv32im-risc0-zkvm-elf` target using the
`risc0` toolchain. Handles linker configuration and cfg flags automatically.

### [stark-v-cli](crates/cli/)

Command-line tool providing the `stark-v` binary. Currently supports:

- `build` - Compile a guest and emit `VmExe` summary

### [prover](crates/prover/)

Proving infrastructure (work in progress). Will contain witness trace generation
and STARK proof construction.

### [stark-v](crates/stark-v/)

Runtime library linked by guest programs. Provides:

- `entry!` macro for defining the guest entry point
- Stack and register initialization
- Custom `terminate` instruction for program exit
- Panic handler for guest targets

### [transpiler](crates/transpiler/)

Converts RV32IM ELF binaries into the `VmExe` format:

- Parses ELF segments and extracts instructions
- Decodes the full RV32IM instruction set plus stark-v custom opcodes
- Builds sparse memory images for VM initialization

## Writing a Guest

Create a new guest crate with `no_std` and `no_main`:

```rust
#![no_std]
#![no_main]

stark_v::entry!(main);

fn main() {
    // Your computation here
}
```

Build it:

```bash
cargo run -- build --guest-path path/to/your/guest
```

## Memory Layout

| Region     | Address       |
| ---------- | ------------- |
| Stack top  | `0x0020_0400` |
| Text start | `0x0020_0800` |

## Requirements

- Rust with the `risc0` toolchain installed
- Target: `riscv32im-risc0-zkvm-elf`

## License

All tools are licensed under the MIT license.
