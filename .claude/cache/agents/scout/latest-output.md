# Codebase Report: stark-v Prover Analysis

Generated: 2026-01-13

## Summary

stark-v is an RV32IM zkVM built on top of Stwo (StarkWare's STARK prover
library) that generates STARK proofs for RISC-V program execution. Its unique
selling propositions center around **CPU-native proving with SIMD optimization**
and **macro-driven rapid constraint development**.

## What is stark-v?

**Core Identity:**

- General-purpose zkVM for RISC-V 32-bit Integer Multiplication (RV32IM)
  instruction set
- Built on Stwo (StarkWare's STARK prover framework)
- Uses declarative macros to generate AIR (Algebraic Intermediate
  Representation) components
- Status: Work in progress, not production-ready

**Architecture:**

```text
Guest Program (RISC-V)
    ↓
Runner (Execution Tracer)
    ↓
Prover (AIR Constraints + Stwo)
    ↓
STARK Proof
```

## Unique Selling Points

### 1. CPU-Native SIMD Proving (The CPU Backend Story)

#### The Key Innovation: 64-byte Aligned SIMD Vectors

Location: `crates/simd/`

stark-v implements custom SIMD infrastructure optimized for CPU proving:

```rust
// crates/simd/src/lib.rs
//! SIMD utilities for aligned vector operations.
//! Provides 64-byte aligned vectors for optimal SIMD performance (AVX-512 / 16x u32).

pub const U32X16_LANES: usize = 16;  // 16 u32 elements per SIMD vector
pub const SIMD_ALIGNMENT: usize = 64; // 64-byte alignment (AVX-512 cache line)
```

**What Makes It Special:**

- `AlignedVec<T>`: Custom vector type with guaranteed 64-byte alignment for SIMD
  ops
- **Zero-copy SIMD conversion**: Directly reinterpret `Vec<u32>` as `&[u32x16]`
  slices
- **Stwo integration**: Converts to Stwo's `BaseColumn` format efficiently
- **Trace table optimization**: All execution trace columns use
  `AlignedVec<u32>`

**Generated code example (from macros):**

```rust
pub struct BaseAluRegTable {
    pub rd_addr: simd::AlignedVec<u32>,
    pub rd_prev: simd::AlignedVec<u32>,
    pub rd_clk_prev: simd::AlignedVec<u32>,
    pub rd_next: simd::AlignedVec<u32>,
    // ... more columns
}
```

### 2. Parallelization Strategy for Maximum Throughput

**Two-pronged approach documented in README:**

#### Strategy 1: Single Proof Parallelism

- Use `--features parallel` (enables Stwo's Rayon)
- Best for: Fast individual proof generation
- Result: ~567 kHz (25M cycles in 44s)

#### Strategy 2: Multiple Non-Parallel Proofs

- Run multiple single-threaded provers in parallel
- Based on research from `rookie-numbers` project
- Best for: Maximum aggregate throughput for recursion scenarios
- Result: Up to ~921 kHz with 12 parallel instances (500K cycles each)

**Allocator Options:**

- `jemalloc`: Up to ~877 kHz throughput
- `mimalloc`: Up to ~932 kHz throughput (best)
- `smalloc`: Experimental, mixed results

**The CPU Proving Insight:** For CPU-native proving (no GPU), running multiple
non-parallel proofs can achieve higher aggregate throughput than single parallel
proofs. This is counterintuitive but validated by benchmarks.

### 3. Macro-Driven Rapid Development

**Three macro systems work together:**

**A. Runner Macros (`crates/stwo-macros/src/trace_tables.rs`)**

`define_trace_tables!` generates:

- Per-opcode trace table structs with typed columns
- `Tracer` aggregating all tables
- `trace_op!` macro for recording execution
- Column accessors for AIR constraints

```rust
define_trace_tables! {
    base_alu_reg: { clk, pc, rd, rs1, rs2, opcode_add_flag, ... },
    lui: { clk, pc, rd, imm_0, imm_1, imm_msb },
    // ... 13+ opcode families
}
```

**B. Prover Macros (`crates/stwo-macros/src/`)**

`relations!` defines LogUp lookup relations:

```rust
relations! {
    relations {
        memory_access: addr_space, addr, clk, limb_0, limb_1, limb_2, limb_3;
        program_access: addr, value_0, value_1, value_2, value_3;
    }
    preprocessed {
        bitwise: a, b, result, op_id;
        range_check_20: value;
    }
}
```

`opcode_components!` aggregates components into proof generation functions.

#### C. LogUp Helper Macros

- `combine!`: Combine columns via LookupElements
- `emit_col!` / `consume_col!`: Write positive/negative fractions
- `add_to_relation!`: Add LogUp constraints in AIR

**Why This Matters:**

- Adding new RISC-V opcodes requires minimal boilerplate
- AIR constraints are declaratively specified
- Type safety across trace generation and constraint checking

### 4. Detailed AIR Documentation

Location: `docs/airs.md` (1656 lines)

**Comprehensive constraint documentation for all 17 component types:**

1. Base ALU Reg (add/sub/xor/or/and)
2. Base ALU Imm (addi/xori/ori/andi)
3. Shifts Reg (sll/srl/sra)
4. Shifts Imm (slli/srli/srai)
5. Less Than Reg (slt/sltu)
6. Less Than Imm (slti/sltiu)
7. Branch Equal (beq/bne)
8. Branch Less Than (blt/bltu/bge/bgeu)
9. LUI
10. AUIPC
11. JALR
12. JAL
13. Load/Store (lb/lbu/lh/lhu/lw/sb/sh/sw)
14. MUL
15. MULH (mulh/mulhsu/mulhu)
16. DIV (div/divu/rem/remu)
17. Preprocessed Range Check 20

Each component documents:

- Columns
- Variables (derived values)
- Constraints (polynomial equations)

### 5. Memory Layout Optimization

Fixed memory layout for guest programs (`guest/guest-bin/linker.ld`):

```text
0x00000400 - 0x000FFFFF  TEXT (rx)      ~1 MB   Program code
0x00100000 - 0x00100FFF  INPUT          4 KB    Input buffer
0x00101000              HALT_FLAG       4 B     Halt detection
0x00101004              OUTPUT_LEN      4 B     Output length
0x00101008 - 0x001FFFBF  OUTPUT         ~1 MB   Output buffer
0x001FFFC0 - 0x001FFFFF  STACK          1 KB    Stack (grows down)
0x00200000 - 0x002FFFFF  DATA (rw)      1 MB    Heap/static data
```

**Design Choice: PC as M31** PC is represented as an M31 field element (not u32)
to reduce overhead. This avoids 3 extra columns per opcode for simple PC
updates.

## Benchmark Results (Apple M2 Max, 12 cores, 64GB RAM)

**Single Parallel Proof (5M Fibonacci):**

- Run: 7831 kHz (3.2s)
- Prove: 567 kHz (44s)
- Run+Prove: 529 kHz (47s total)

**Multiple Non-Parallel Proofs (500K Fibonacci each):**

- 8 parallel: 747 kHz
- 10 parallel: 843 kHz
- 12 parallel: 921 kHz (best throughput)

**With mimalloc (best allocator):**

- 12 parallel: 932 kHz (fastest)

## Architecture Map

```text
[Guest Program]
    ↓ (compiled to RISC-V ELF)
[Runner] - Executes and traces
    ↓ (generates execution trace)
[Tracer] - Records trace tables (AlignedVec columns)
    ↓ (converts to BaseColumn)
[Prover] - Applies AIR constraints via Stwo
    ↓ (generates STARK proof)
[Verifier] - Verifies proof
```

## Key Files

| File                                     | Purpose                | Key Features                         |
| ---------------------------------------- | ---------------------- | ------------------------------------ |
| `crates/simd/src/aligned_vec.rs`         | SIMD-aligned vectors   | 64-byte alignment, u32x16 conversion |
| `crates/simd/src/allocator.rs`           | Custom allocator       | Ensures 64-byte alignment            |
| `crates/stwo-macros/src/trace_tables.rs` | Trace table generation | `define_trace_tables!` macro         |
| `crates/runner/src/cpu.rs`               | CPU execution          | RISC-V instruction interpreter       |
| `crates/prover/src/prover.rs`            | Proof generation       | AIR component orchestration          |
| `docs/airs.md`                           | AIR documentation      | All 17 component constraints         |

## Codebase Structure

```text
stark-v/
├── crates/
│   ├── simd/           # Custom SIMD infrastructure (CPU backend optimization)
│   ├── stwo-macros/    # Proc-macros for trace tables and AIR components
│   ├── runner/         # RISC-V execution tracer
│   ├── prover/         # STARK proof generation
│   ├── sdk/            # Public API for external consumers
│   └── debug-utils/    # Debugging and visualization
├── guest/
│   ├── guest-lib/      # Guest program SDK
│   └── guest-bin/      # Example guest programs + linker script
├── external/stwo/      # Stwo STARK prover (submodule)
└── docs/airs.md        # Detailed constraint documentation
```

## Comparison to Other STARK Provers

**What makes stark-v different:**

1. **CPU-First Design**: Optimized for CPU proving with SIMD, not GPU-first
2. **Macro-Driven AIR**: Uses Rust proc-macros for rapid constraint development
3. **Memory-Aligned Traces**: All trace columns use 64-byte aligned vectors
4. **Parallelization Research**: Documents two proven strategies for max
   throughput
5. **Stwo Integration**: Leverages StarkWare's production-grade STARK library
6. **RV32IM Focus**: Full RISC-V Integer + Multiplication support (no compressed
   instructions yet)

**Inspired by OpenVM:** The AIR component design is inspired by OpenVM's
architecture (acknowledged in README).

## Features

| Feature      | Purpose                                 |
| ------------ | --------------------------------------- |
| `parallel`   | Enable Stwo's Rayon parallelism         |
| `jemalloc`   | Use jemalloc allocator (~877 kHz)       |
| `mimalloc`   | Use mimalloc allocator (~932 kHz, best) |
| `smalloc`    | Use smalloc allocator (experimental)    |
| `peak-alloc` | Track peak memory usage                 |

## Open Questions & Future Work

- Production readiness timeline?
- GPU backend support plans?
- Compressed RISC-V instructions (RV32IMC)?
- Recursive proof composition strategy?
- Comparison with other zkVMs (SP1, Risc0, Jolt)?

## Conclusion

**The CPU Backend USP:**

stark-v's "CPU backend" isn't a fallback—it's the primary design target. The
custom SIMD infrastructure (`AlignedVec<u32>` with 64-byte alignment) is
purpose-built for AVX-512 CPU proving. Combined with the parallelization
strategy of running multiple non-parallel provers, stark-v achieves competitive
throughput on commodity CPU hardware.

**Key Differentiators:**

1. 64-byte aligned SIMD vectors for CPU proving
2. Macro-driven rapid AIR development
3. Proven parallelization strategies (up to 932 kHz on M2 Max)
4. Built on Stwo (production-grade STARK library)
5. Comprehensive AIR documentation (1656 lines)

The project is a practical demonstration that CPU-native proving can be
competitive when combined with proper SIMD optimization and parallelization
strategy.
