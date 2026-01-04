# stark-v

A general purpose zkVM building on top of
[Stwo](https://github.com/starkware-libs/stwo).

## Overview

stark-v is an RV32IM zkVM that generates STARK proofs for RISC-V program
execution. The prover uses declarative macros to generate Stwo AIR components,
enabling rapid development of new constraints.

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
    // Record trace row: opcode flags select which constraint applies
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
let proof = prove_rv32im(run_result, PcsConfig::default());
verify_rv32im(proof, PcsConfig::default())?;
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
   Compiling prover v0.1.0 (/Users/clementwalter/Documents/starkware/stark-v/crates/prover)
    Finished `release` profile [optimized] target(s) in 1.78s
     Running tests/integration.rs (target/release/deps/integration-4acc7c147dc4d264)

running 1 test
    Finished `release` profile [optimized] target(s) in 0.06s
2026-01-04T14:02:02.034985Z  INFO Generate traces: prover::prover: Tracer total_traces: 25003428
2026-01-04T14:02:04.814830Z  INFO Generate traces: prover::prover: Max trace log_size: 24
2026-01-04T14:02:04.965158Z  INFO Preprocessed trace: prover::prover: Preprocessed trace ids len: 14
2026-01-04T14:02:05.370744Z  INFO Main trace: prover::prover: Main trace columns committed: 1057
2026-01-04T14:02:10.077151Z  INFO prover::prover: proof of work with 10 bits
2026-01-04T14:02:46.606295Z  INFO Prove:prove_ex: stwo::prover: proof_size_estimate=83396
2026-01-04T14:02:46.788014Z  INFO stwo::core::verifier: Composition polynomial log degree bound: 25
2026-01-04T14:02:46.788219Z  INFO stwo::core::verifier: Sampling 1579 columns.
2026-01-04T14:02:46.788225Z  INFO stwo::core::verifier: Total sample points: 1691.
2026-01-04T14:02:46.790615Z  INFO integration: fib_input benchmark
2026-01-04T14:02:46.790623Z  INFO integration:   n: 5000000
2026-01-04T14:02:46.790625Z  INFO integration:   cycles: 25000170
2026-01-04T14:02:46.790626Z  INFO integration:   run:       7599.932 kHz  (3.290s)
2026-01-04T14:02:46.790630Z  INFO integration:   run+prove:    520.441 kHz  (48.037s)
2026-01-04T14:02:46.790633Z  INFO integration:   prove:        558.701 kHz  (44.747s)
test test_e2e_fibonacci_benchmark ... ok
```

- Benchmark several proofs generations in parallel (max throughput for
  continuation/recursion):

```sh
cargo bench --package prover --bench fibonacci
Timer precision: 41 ns
fibonacci           fastest       │ slowest       │ median        │ mean          │ samples │ iters
╰─ bench_fibonacci                │               │               │               │         │
   ├─ 100000                      │               │               │               │         │
   │  ├─ 1          10.43 s       │ 10.43 s       │ 10.43 s       │ 10.43 s       │ 1       │ 1
   │  │             47.91 Kitem/s │ 47.91 Kitem/s │ 47.91 Kitem/s │ 47.91 Kitem/s │         │
   │  ├─ 4          11.13 s       │ 11.13 s       │ 11.13 s       │ 11.13 s       │ 1       │ 1
   │  │             179.6 Kitem/s │ 179.6 Kitem/s │ 179.6 Kitem/s │ 179.6 Kitem/s │         │
   │  ├─ 8          11.91 s       │ 11.91 s       │ 11.91 s       │ 11.91 s       │ 1       │ 1
   │  │             335.7 Kitem/s │ 335.7 Kitem/s │ 335.7 Kitem/s │ 335.7 Kitem/s │         │
   │  ╰─ 12         14.35 s       │ 14.35 s       │ 14.35 s       │ 14.35 s       │ 1       │ 1
   │                418.1 Kitem/s │ 418.1 Kitem/s │ 418.1 Kitem/s │ 418.1 Kitem/s │         │
   ├─ 500000                      │               │               │               │         │
   │  ├─ 1          25.31 s       │ 25.31 s       │ 25.31 s       │ 25.31 s       │ 1       │ 1
   │  │             98.77 Kitem/s │ 98.77 Kitem/s │ 98.77 Kitem/s │ 98.77 Kitem/s │         │
   │  ├─ 4          25.96 s       │ 25.96 s       │ 25.96 s       │ 25.96 s       │ 1       │ 1
   │  │             385.1 Kitem/s │ 385.1 Kitem/s │ 385.1 Kitem/s │ 385.1 Kitem/s │         │
   │  ├─ 8          29.27 s       │ 29.27 s       │ 29.27 s       │ 29.27 s       │ 1       │ 1
   │  │             683.2 Kitem/s │ 683.2 Kitem/s │ 683.2 Kitem/s │ 683.2 Kitem/s │         │
   │  ╰─ 12         42.74 s       │ 42.74 s       │ 42.74 s       │ 42.74 s       │ 1       │ 1
   │                701.8 Kitem/s │ 701.8 Kitem/s │ 701.8 Kitem/s │ 701.8 Kitem/s │         │
   ├─ 1000000                     │               │               │               │         │
   │  ├─ 1          44.44 s       │ 44.44 s       │ 44.44 s       │ 44.44 s       │ 1       │ 1
   │  │             112.4 Kitem/s │ 112.4 Kitem/s │ 112.4 Kitem/s │ 112.4 Kitem/s │         │
   │  ├─ 4          46.36 s       │ 46.36 s       │ 46.36 s       │ 46.36 s       │ 1       │ 1
   │  │             431.3 Kitem/s │ 431.3 Kitem/s │ 431.3 Kitem/s │ 431.3 Kitem/s │         │
   │  ├─ 8          1.125 m       │ 1.125 m       │ 1.125 m       │ 1.125 m       │ 1       │ 1
   │  │             592.2 Kitem/s │ 592.2 Kitem/s │ 592.2 Kitem/s │ 592.2 Kitem/s │         │
   │  ╰─ 12         2.293 m       │ 2.293 m       │ 2.293 m       │ 2.293 m       │ 1       │ 1
```

With `--features jemalloc`:

```sh
cargo bench --package prover --bench fibonacci --features jemalloc
Timer precision: 41 ns
fibonacci           fastest       │ slowest       │ median        │ mean          │ samples │ iters
╰─ bench_fibonacci                │               │               │               │         │
   ├─ 500000                      │               │               │               │         │
   │  ├─ 8          26.58 s       │ 26.58 s       │ 26.58 s       │ 26.58 s       │ 1       │ 1
   │  │             752.3 Kitem/s │ 752.3 Kitem/s │ 752.3 Kitem/s │ 752.3 Kitem/s │         │
   │  ├─ 10         30.42 s       │ 30.42 s       │ 30.42 s       │ 30.42 s       │ 1       │ 1
   │  │             821.7 Kitem/s │ 821.7 Kitem/s │ 821.7 Kitem/s │ 821.7 Kitem/s │         │
   │  ╰─ 12         37.84 s       │ 37.84 s       │ 37.84 s       │ 37.84 s       │ 1       │ 1
   │                792.7 Kitem/s │ 792.7 Kitem/s │ 792.7 Kitem/s │ 792.7 Kitem/s │         │
```

With `--features smalloc`:

```sh
cargo bench --package prover --bench fibonacci --features smalloc
Timer precision: 41 ns
fibonacci           fastest       │ slowest       │ median        │ mean          │ samples │ iters
╰─ bench_fibonacci                │               │               │               │         │
   ├─ 500000                      │               │               │               │         │
   │  ├─ 8          26.94 s       │ 26.94 s       │ 26.94 s       │ 26.94 s       │ 1       │ 1
   │  │             742.4 Kitem/s │ 742.4 Kitem/s │ 742.4 Kitem/s │ 742.4 Kitem/s │         │
   │  ├─ 10         33.76 s       │ 33.76 s       │ 33.76 s       │ 33.76 s       │ 1       │ 1
   │  │             740.5 Kitem/s │ 740.5 Kitem/s │ 740.5 Kitem/s │ 740.5 Kitem/s │         │
   │  ╰─ 12         52.52 s       │ 52.52 s       │ 52.52 s       │ 52.52 s       │ 1       │ 1
   │                571.1 Kitem/s │ 571.1 Kitem/s │ 571.1 Kitem/s │ 571.1 Kitem/s │         │
```

With `--features mimalloc`:

```sh
cargo bench --package prover --bench fibonacci --features mimalloc
Timer precision: 41 ns
fibonacci           fastest       │ slowest       │ median        │ mean          │ samples │ iters
╰─ bench_fibonacci                │               │               │               │         │
   ├─ 500000                      │               │               │               │         │
   │  ├─ 8          26.4 s        │ 26.4 s        │ 26.4 s        │ 26.4 s        │ 1       │ 1
   │  │             757.5 Kitem/s │ 757.5 Kitem/s │ 757.5 Kitem/s │ 757.5 Kitem/s │         │
   │  ├─ 10         29.44 s       │ 29.44 s       │ 29.44 s       │ 29.44 s       │ 1       │ 1
   │  │             848.9 Kitem/s │ 848.9 Kitem/s │ 848.9 Kitem/s │ 848.9 Kitem/s │         │
   │  ╰─ 12         36.8 s        │ 36.8 s        │ 36.8 s        │ 36.8 s        │ 1       │ 1
   │                815.2 Kitem/s │ 815.2 Kitem/s │ 815.2 Kitem/s │ 815.2 Kitem/s │         │
```

## Features

- `parallel` - Enable Stwo's Rayon parallelism
- `jemalloc` - Use jemalloc allocator
- `mimalloc` - Use mimalloc allocator
- `smalloc` - Use smalloc allocator
- `peak-alloc` - Track peak memory usage

## License

See [LICENSE](LICENSE) for details.
