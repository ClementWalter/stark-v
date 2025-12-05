# Playground Guest Program

A minimal RISC-V guest program for the risc0 zkVM target.

## Prerequisites

### Install the risc0 toolchain

The risc0 toolchain provides a custom Rust compiler that targets the
`riscv32im-risc0-zkvm-elf` architecture:

```bash
# Install rzup (risc0's toolchain manager)
curl -L https://risczero.com/install | bash

# Install the risc0 toolchain
rzup install
```

This installs a custom rustc channel called `risc0` which is configured in
`rust-toolchain.toml`.

### Install LLVM tools (for inspection)

To inspect the compiled ELF binary, you'll need `llvm-objdump`:

```bash
# macOS
brew install llvm

# Ubuntu/Debian
sudo apt install llvm
```

## Building

From the `guests/playground/` directory:

```bash
# Build for the risc0 zkVM target
cargo build --target riscv32im-risc0-zkvm-elf

# Or build in release mode
cargo build --target riscv32im-risc0-zkvm-elf --release
```

The compiled ELF binary will be at:

- Debug: `target/riscv32im-risc0-zkvm-elf/debug/playground`
- Release: `target/riscv32im-risc0-zkvm-elf/release/playground`

## Inspecting the ELF Binary

### Disassemble the entire binary

```bash
llvm-objdump -d target/riscv32im-risc0-zkvm-elf/debug/playground
```

### Example output

```text
target/riscv32im-risc0-zkvm-elf/debug/playground:	file format elf32-littleriscv

Disassembly of section .text:

000000b0 <main>:
      b0: 13 01 01 ff   addi    sp, sp, -16
      b4: 23 26 11 00   sw      ra, 12(sp)
      b8: 13 05 40 00   li      a0, 4
      bc: 23 24 a1 00   sw      a0, 8(sp)
      c0: 13 05 50 00   li      a0, 5
      c4: 23 22 a1 00   sw      a0, 4(sp)
      c8: 13 05 90 00   li      a0, 9
      cc: 23 20 a1 00   sw      a0, 0(sp)
      d0: 83 20 c1 00   lw      ra, 12(sp)
      d4: 13 01 01 01   addi    sp, sp, 16
      d8: 67 80 00 00   ret
```

### Useful inspection commands

```bash
# View only the main function
llvm-objdump -d --disassemble-symbols=main target/riscv32im-risc0-zkvm-elf/debug/playground

# Show section headers
llvm-objdump -h target/riscv32im-risc0-zkvm-elf/debug/playground

# Show all symbols
llvm-objdump -t target/riscv32im-risc0-zkvm-elf/debug/playground

# Show file headers (entry point, architecture, etc.)
llvm-objdump -f target/riscv32im-risc0-zkvm-elf/debug/playground

# Pretty print with source interleaved (if debug info available)
llvm-objdump -d -S target/riscv32im-risc0-zkvm-elf/debug/playground

# Using readelf for ELF structure
llvm-readelf -a target/riscv32im-risc0-zkvm-elf/debug/playground
```

## Understanding the Output

The `riscv32im-risc0-zkvm-elf` target produces a 32-bit RISC-V ELF binary with:

- **Architecture**: RV32IM (32-bit base integer + multiply/divide extension)
- **ABI**: ILP32 (32-bit integers, longs, and pointers)
- **Format**: Little-endian ELF

Key RISC-V registers:

- `sp` (x2): Stack pointer
- `ra` (x1): Return address
- `a0-a7` (x10-x17): Function arguments and return values

## Program Structure

This is a `#![no_std]` program with:

- No standard library (bare metal)
- No main runtime (`#![no_main]`)
- Custom panic handler (infinite loop)
- C-compatible `main` entry point

The zkVM executor will load this ELF and execute it instruction by instruction,
generating a cryptographic proof of correct execution.
