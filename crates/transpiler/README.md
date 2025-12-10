# transpiler

ELF to VmExe transpiler for RISC-V programs.

## Overview

The `transpiler` crate converts RV32IM ELF binaries into the `VmExe` format used
by the stark-v VM. It parses ELF segments, decodes RISC-V instructions, and
translates them into the VM's internal instruction representation.

## Features

- **ELF parsing** - Loads 32-bit RISC-V executables
- **Instruction decoding** - Supports the full RV32IM instruction set
- **Custom opcode support** - Handles stark-v specific instructions (terminate,
  phantom, hints)
- **Memory image extraction** - Builds sparse initial memory from ELF segments

## Usage

```rust
use std::path::Path;
use transpiler::VmExe;

let exe = VmExe::from_path(Path::new("path/to/guest.elf"))?;
println!("Instructions: {}", exe.program.len());
println!("PC start: 0x{:08x}", exe.pc_start);
println!("Init memory bytes: {}", exe.init_memory.len());
```

## Architecture

### VmExe

The main output type containing:

- `program` - Vector of transpiled `Instruction`s with base PC
- `pc_start` - Entry point address
- `init_memory` - Sparse memory image as `BTreeMap<(address_space, addr), byte>`

### Instruction Format

Each VM instruction contains:

| Field    | Description                                   |
| -------- | --------------------------------------------- |
| `opcode` | VM opcode (see opcode groups below)           |
| `a`-`g`  | Operand fields (registers, immediates, flags) |

### Opcode Groups

| Group     | Opcodes                             | Base  |
| --------- | ----------------------------------- | ----- |
| System    | TERMINATE, PHANTOM                  | 0x000 |
| BaseAlu   | ADD, SUB, XOR, OR, AND              | 0x200 |
| Shift     | SLL, SRL, SRA                       | 0x205 |
| LessThan  | SLT, SLTU                           | 0x208 |
| LoadStore | LOADW, LOADBU, LOADHU, STOREW, etc. | 0x210 |
| Branch    | BEQ, BNE, BLT, BGE, BLTU, BGEU      | 0x220 |
| Jump      | JAL, LUI, JALR, AUIPC               | 0x230 |
| Mul       | MUL, MULH, MULHSU, MULHU            | 0x250 |
| DivRem    | DIV, DIVU, REM, REMU                | 0x254 |
| Hint      | HINT_STOREW, HINT_BUFFER            | 0x260 |

## Custom Instructions

The transpiler recognizes stark-v custom instructions encoded in RISC-V:

- **TERMINATE** - Halts execution with an exit code
- **PHANTOM** - Hints and intrinsics (input, print, random)
- **REVEAL** - Writes to public output memory
- **HINT_STOREW/BUFFER** - Loads hint data

## Error Handling

The `RunnerError` type covers:

- ELF format validation (32-bit, RISC-V, executable)
- Memory bounds checking
- Unsupported instruction detection
- I/O failures

## Credits

Initial implementation inspired by
[OpenVM](https://github.com/openvm-org/openvm) transpiler (MIT/Apache-2.0
licensed).
