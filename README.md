# stark-v

A general purpose zkVM building on top of
[Stwo](https://github.com/starkware-libs/stwo).

## Overview

stark-v is an RV32IM zkVM that generates STARK proofs for RISC-V program
execution. The prover uses declarative macros to generate Stwo AIR components,
enabling rapid development of new constraints.

:warning: This is a work in progress and not yet ready for production.

### Credits

The AIR component design is inspired by
[OpenVM](https://github.com/openvm-org/openvm).

## Architecture

### Runner Macros

The `runner-macros` crate provides `define_trace_tables!` for generating
execution trace infrastructure:

**`define_trace_tables!`** - Defines columnar trace tables and generates:

- Per-opcode `Table` structs with typed columns (e.g., `BaseAluRegTable`)
- `Tracer` struct aggregating all opcode tables
- `trace_op!` macro for recording execution traces
- `prover_columns` module with column accessors for AIR constraints

```rust
define_trace_tables! {
    base_alu_reg: {
        clk, pc, rd, rs1, rs2,
        opcode_add_flag, opcode_sub_flag, opcode_xor_flag, opcode_or_flag, opcode_and_flag
    },
    lui: { clk, pc, rd, imm_0, imm_1, imm_msb },
    load_store: { clk, pc, rd, rs1, mem, ... },
}
```

**`trace_op!`** - Records opcode execution during VM run:

```rust
// In opcode implementation (e.g., ops/alu.rs)
pub fn add(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let result = rs1.next.wrapping_add(rs2.next);
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    // Record trace row in the given table with any required columns
    trace_op!(base_alu_reg: tracer, old_pc, rd, rs1, rs2, 1, 0, 0, 0, 0);
}
```

The `Access` type captures register/memory state transitions with `prev`/`next`
values for continuity constraints.

### Memory Layout

The guest program uses a fixed memory layout defined in
`guest/guest-bin/linker.ld`:

```text
Address Range           Region          Size
─────────────────────────────────────────────────
0x00000400 - 0x000FFFFF  TEXT (rx)      ~1 MB   Program code
0x00100000 - 0x00100FFF  INPUT          4 KB    Input buffer
0x00101000              HALT_FLAG       4 B     Halt detection
0x00101004              OUTPUT_LEN      4 B     Output length
0x00101008 - 0x001FFFBF  OUTPUT         ~1 MB   Output buffer
0x001FFFC0 - 0x001FFFFF  STACK          1 KB    Stack (grows down)
0x00200000 - 0x002FFFFF  DATA (rw)      1 MB    Heap/static data
```

### Component Macros

The prover uses three main macros to generate Stwo AIR infrastructure:

**`relations!`** - Defines LogUp lookup relations and generates:

- Wrapper types implementing `Relation<F, EF>` trait
- `Relations` struct containing all lookup elements
- `PreProcessedTrace` for constant lookup tables
- `Counters` for multiplicity tracking

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

**`opcode_components!`** - Aggregates RV32IM opcode components into:

- `Traces` struct with columns per opcode family
- `Claim` and `ClaimedSum` for proof claims
- `Components` struct with AIR component instances
- `gen_trace()` and `gen_interaction_trace()` functions

**LogUp helper macros** - Simplify interaction trace generation:

- `combine!` - Combine columns via LookupElements
- `emit_col!` / `consume_col!` - Write positive/negative fractions
- `add_to_relation!` - Add LogUp constraints in AIR

## Usage

### Writing a Guest Program

Create a guest binary using the `guest_main!` macro:

```rust
// guest/my-program/src/main.rs
#![no_std]
#![no_main]

guest_bin::guest_main!({
    // Your computation here
    let result = 42u32;
    result
});
```

### Proving Execution

```rust
use prover::{prove_rv32im, verify_rv32im};
use runner::run_with_input;
use stwo::core::pcs::PcsConfig;

// Load and run the guest ELF
let elf_bytes = std::fs::read("path/to/guest.elf")?;
let input = 42u32.to_le_bytes();
let run_result = run_with_input(&elf_bytes, &input, 100_000_000)?;

// Generate and verify proof
let config = PcsConfig::default();
let preprocessed = prover::preprocess(config);
let proof = prove_rv32im(run_result, config, &preprocessed);
verify_rv32im(proof, config, &preprocessed)?;
```

## Benchmarks

The benchmark measures proving throughput in kHz (thousands of cycles per
second).

### Parallelization Strategy

Two approaches to maximize throughput:

1. **Stwo `parallel` feature**: Uses Rayon for intra-proof parallelism. Best for
   individual proof latency.

2. **Multiple non-parallel proofs**: Run multiple single-threaded provers in
   parallel. Based on findings from
   [rookie-numbers](https://github.com/clementwalter/rookie-numbers/), this can
   achieve higher aggregate throughput, useful for recursion scenarios.

### Running Benchmarks

```bash
# Clone with submodules
git clone --recursive https://github.com/starkware-libs/stark-v.git
cd stark-v

# Non-parallel Stwo with parallel processes (max throughput)
cargo bench --package prover --bench fibonacci

# Parallel Stwo (faster individual proofs)
cargo bench --package prover --bench fibonacci --features parallel

# With jemalloc allocator
cargo bench --package prover --bench fibonacci --features "parallel,jemalloc"
```

### Results

Measured on Apple M2 Max with 12 physical cores and 64GB of RAM:

- Single test with parallel features:

```sh
STARKV_FIB_N=5000000 cargo test --release --package prover --features parallel --test integration -- test_e2e_fibonacci_benchmark --exact --nocapture

running 1 test
    Finished `release` profile [optimized] target(s) in 0.06s
2026-01-04T14:34:44.089476Z  INFO Generate traces: prover::prover: Tracer total_traces: 25003428
2026-01-04T14:34:46.758412Z  INFO Generate traces: prover::prover: Max trace log_size: 24
2026-01-04T14:34:46.905371Z  INFO Preprocessed trace: prover::prover: Preprocessed trace ids len: 14
2026-01-04T14:34:47.303413Z  INFO Main trace: prover::prover: Main trace columns committed: 1057
2026-01-04T14:34:51.971467Z  INFO prover::prover: proof of work with 10 bits
2026-01-04T14:35:27.999387Z  INFO Prove:prove_ex: stwo::prover: proof_size_estimate=83396
2026-01-04T14:35:28.166420Z  INFO stwo::core::verifier: Composition polynomial log degree bound: 25
2026-01-04T14:35:28.166594Z  INFO stwo::core::verifier: Sampling 1579 columns.
2026-01-04T14:35:28.166600Z  INFO stwo::core::verifier: Total sample points: 1691.
2026-01-04T14:35:28.169360Z  INFO integration: fib_input benchmark
2026-01-04T14:35:28.169368Z  INFO integration:   n: 5000000
2026-01-04T14:35:28.169369Z  INFO integration:   cycles: 25000170
2026-01-04T14:35:28.169370Z  INFO integration:   run:       7831.143 kHz  (3.192s)
2026-01-04T14:35:28.169375Z  INFO integration:   run+prove:    528.939 kHz  (47.265s)
2026-01-04T14:35:28.169377Z  INFO integration:   prove:        567.253 kHz  (44.072s)
test test_e2e_fibonacci_benchmark ... ok
```

- Benchmark several proofs generations in parallel (max throughput for
  continuation/recursion):

```sh
cargo bench --package prover --bench fibonacci
Timer precision: 41 ns
fibonacci           fastest       │ slowest       │ median        │ mean          │ samples │ iters
╰─ bench_fibonacci                │               │               │               │         │
   ├─ 500000                      │               │               │               │         │
   │  ├─ 8          26.75 s       │ 26.75 s       │ 26.75 s       │ 26.75 s       │ 1       │ 1
   │  │             747.4 Kitem/s │ 747.4 Kitem/s │ 747.4 Kitem/s │ 747.4 Kitem/s │         │
   │  ├─ 10         29.65 s       │ 29.65 s       │ 29.65 s       │ 29.65 s       │ 1       │ 1
   │  │             843.1 Kitem/s │ 843.1 Kitem/s │ 843.1 Kitem/s │ 843.1 Kitem/s │         │
   │  ╰─ 12         32.57 s       │ 32.57 s       │ 32.57 s       │ 32.57 s       │ 1       │ 1
   │                921 Kitem/s   │ 921 Kitem/s   │ 921 Kitem/s   │ 921 Kitem/s   │         │
   ├─ 750000                      │               │               │               │         │
   │  ├─ 8          55.85 s       │ 55.85 s       │ 55.85 s       │ 55.85 s       │ 1       │ 1
   │  │             537 Kitem/s   │ 537 Kitem/s   │ 537 Kitem/s   │ 537 Kitem/s   │         │
   │  ├─ 10         1.186 m       │ 1.186 m       │ 1.186 m       │ 1.186 m       │ 1       │ 1
   │  │             526.7 Kitem/s │ 526.7 Kitem/s │ 526.7 Kitem/s │ 526.7 Kitem/s │         │
```

With `--features jemalloc`:

```sh
cargo bench --package prover --bench fibonacci --features jemalloc
Timer precision: 41 ns
fibonacci           fastest       │ slowest       │ median        │ mean          │ samples │ iters
╰─ bench_fibonacci                │               │               │               │         │
   ├─ 500000                      │               │               │               │         │
   │  ├─ 8          25.65 s       │ 25.65 s       │ 25.65 s       │ 25.65 s       │ 1       │ 1
   │  │             779.7 Kitem/s │ 779.7 Kitem/s │ 779.7 Kitem/s │ 779.7 Kitem/s │         │
   │  ├─ 10         28.48 s       │ 28.48 s       │ 28.48 s       │ 28.48 s       │ 1       │ 1
   │  │             877.5 Kitem/s │ 877.5 Kitem/s │ 877.5 Kitem/s │ 877.5 Kitem/s │         │
   │  ╰─ 12         35.4 s        │ 35.4 s        │ 35.4 s        │ 35.4 s        │ 1       │ 1
   │                847.4 Kitem/s │ 847.4 Kitem/s │ 847.4 Kitem/s │ 847.4 Kitem/s │         │
   ├─ 750000                      │               │               │               │         │
   │  ├─ 8          59 s          │ 59 s          │ 59 s          │ 59 s          │ 1       │ 1
   │  │             508.4 Kitem/s │ 508.4 Kitem/s │ 508.4 Kitem/s │ 508.4 Kitem/s │         │
```

With `--features smalloc`:

```sh
cargo bench --package prover --bench fibonacci --features smalloc
fibonacci           fastest       │ slowest       │ median        │ mean          │ samples │ iters
╰─ bench_fibonacci                │               │               │               │         │
   ├─ 500000                      │               │               │               │         │
   │  ├─ 8          25.45 s       │ 25.45 s       │ 25.45 s       │ 25.45 s       │ 1       │ 1
   │  │             785.7 Kitem/s │ 785.7 Kitem/s │ 785.7 Kitem/s │ 785.7 Kitem/s │         │
   │  ├─ 10         29.76 s       │ 29.76 s       │ 29.76 s       │ 29.76 s       │ 1       │ 1
   │  │             839.9 Kitem/s │ 839.9 Kitem/s │ 839.9 Kitem/s │ 839.9 Kitem/s │         │
   │  ╰─ 12         36.3 s        │ 36.3 s        │ 36.3 s        │ 36.3 s        │ 1       │ 1
   │                826.4 Kitem/s │ 826.4 Kitem/s │ 826.4 Kitem/s │ 826.4 Kitem/s │         │
   ├─ 750000                      │               │               │               │         │
   │  ├─ 8          1.254 m       │ 1.254 m       │ 1.254 m       │ 1.254 m       │ 1       │ 1
   │  │             398.6 Kitem/s │ 398.6 Kitem/s │ 398.6 Kitem/s │ 398.6 Kitem/s │         │
   │  ├─ 10         2.895 m       │ 2.895 m       │ 2.895 m       │ 2.895 m       │ 1       │ 1
```

With `--features mimalloc`:

```sh
cargo bench --package prover --bench fibonacci --features mimalloc
Timer precision: 41 ns
fibonacci           fastest       │ slowest       │ median        │ mean          │ samples │ iters
╰─ bench_fibonacci                │               │               │               │         │
   ├─ 500000                      │               │               │               │         │
   │  ├─ 8          25.67 s       │ 25.67 s       │ 25.67 s       │ 25.67 s       │ 1       │ 1
   │  │             778.9 Kitem/s │ 778.9 Kitem/s │ 778.9 Kitem/s │ 778.9 Kitem/s │         │
   │  ├─ 10         28.43 s       │ 28.43 s       │ 28.43 s       │ 28.43 s       │ 1       │ 1
   │  │             879.2 Kitem/s │ 879.2 Kitem/s │ 879.2 Kitem/s │ 879.2 Kitem/s │         │
   │  ╰─ 12         32.17 s       │ 32.17 s       │ 32.17 s       │ 32.17 s       │ 1       │ 1
   │                932.4 Kitem/s │ 932.4 Kitem/s │ 932.4 Kitem/s │ 932.4 Kitem/s │         │
   ├─ 750000                      │               │               │               │         │
   │  ├─ 8          53.91 s       │ 53.91 s       │ 53.91 s       │ 53.91 s       │ 1       │ 1
   │  │             556.4 Kitem/s │ 556.4 Kitem/s │ 556.4 Kitem/s │ 556.4 Kitem/s │         │
   │  ├─ 10         1.182 m       │ 1.182 m       │ 1.182 m       │ 1.182 m       │ 1       │ 1
   │  │             528.5 Kitem/s │ 528.5 Kitem/s │ 528.5 Kitem/s │ 528.5 Kitem/s │         │
```

## Features

- `parallel` - Enable Stwo's Rayon parallelism
- `jemalloc` - Use jemalloc allocator
- `mimalloc` - Use mimalloc allocator
- `smalloc` - Use smalloc allocator
- `peak-alloc` - Track peak memory usage

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
