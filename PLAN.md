## Goal

Add a lightweight ELF-to-VM executable pipeline (similar to OpenVM’s
`Elf::decode` and `VmExe::from_elf`) directly inside this repository
(`stark-v`). The new CLI must allow `cargo run -- run-elf --path <guest-elf>` to
parse a RISC-V guest ELF, transpile it using only the RV32IM extension (via
`rrs-lib`), and emit a `VmExe` struct containing:

1. A `Program` (vector of OpenVM-style `Instruction`/debug tuples) produced by
   transpiling decoded ELF instructions.
2. `pc_start` (ELF entry point).
3. `init_memory` as a sparse map keyed by `(segment_id, offset)` (convert the
   ELF’s `BTreeMap<u32,u32>` memory image to `BTreeMap<(u32,u32),u8>`).

Success criterion: running
`cargo run -- run-elf --path <path/to/guests/playground/target/riscv32im-risc0-zkvm-elf/release/playground>`
produces a valid `VmExe` (no need to execute it yet).

---

## Required References

- The complete OpenVM repository is available at `../openvm`. Key files to
  cite/replicate:
  - `../openvm/crates/toolchain/transpiler/src/elf.rs`
  - `../openvm/crates/toolchain/transpiler/src/util.rs`
  - `../openvm/extensions/rv32im/transpiler/src/lib.rs`
  - `../openvm/crates/toolchain/instructions/src/{exe.rs, program.rs, instruction.rs}`
  - `../openvm/crates/toolchain/platform/src/memory.rs` (for memory layout
    constants)
  - `../openvm/crates/cli/src/commands/build.rs` (demonstrates how `VmExe` is
    produced)

## Detailed Plan

1. **Audit structure**
   - From the current `stark-v` repo root, inspect `src/main.rs` (CLI entry) and
     `crates/runner` to learn how commands are registered.
   - Note existing data structures we can reuse; otherwise plan to add minimal
     versions for `VmExe`, `Program`, `Instruction`.

2. **Define shared VM data structures**
   - In `crates/runner/src` create/update module(s) defining:
     - `Instruction` (fields modeled after
       `../openvm/crates/toolchain/instructions/src/instruction.rs`).
     - `Program` (vector of optional `(Instruction, Option<DebugInfo>)` or
       simplified variant).
     - `VmExe { program, pc_start, init_memory: BTreeMap<(u32,u32), u8> }`
       referencing `../openvm/crates/toolchain/instructions/src/exe.rs`.
   - Include helper constructors (e.g., `Instruction::from_r_type`) copied from
     `../openvm/crates/toolchain/transpiler/src/util.rs`, attributing source in
     comments.

3. **Implement ELF decoding module**
   - Add `crates/runner/src/elf.rs` with:
     - `pub struct Elf { instructions: Vec<u32>, pc_start: u32, pc_base: u32, memory_image: BTreeMap<u32,u32> }`.
     - `impl Elf { pub fn from_path(path: &Path, max_mem: u32) -> eyre::Result<Self> }`.
   - Base implementation on `../openvm/crates/toolchain/transpiler/src/elf.rs`,
     including loadable segment iteration, `.bss` zero fill, max memory checks,
     function span logic omitted.
   - Comment referencing the OpenVM file (per user request).

4. **Implement RV32-only transpiler**
   - Add `crates/runner/src/transpiler.rs` containing
     `pub fn transpile_elf(elf: Elf) -> eyre::Result<VmExe>`.
   - Reuse logic from `../openvm/extensions/rv32im/transpiler/src/lib.rs` and
     `../openvm/crates/toolchain/transpiler/src/util.rs` to map RV32
     instructions to our `Instruction`.
   - Use `rrs-lib` for decoding opcodes, but drop references to other OpenVM
     extensions.
   - Convert `memory_image: BTreeMap<u32,u32>` into sparse
     `(segment_id, offset)` map (use constant `const RV32_MEMORY_AS: u32 = 2`,
     referencing `../openvm/crates/toolchain/instructions/src/riscv.rs`).

5. **Expose runner API**
   - In `crates/runner/src/lib.rs`, expose
     `pub fn load_vmexe_from_elf(path: &Path) -> eyre::Result<VmExe>`:
     1. Calls `Elf::from_path(path, MAX_MEM)` (MAX_MEM from
        `../openvm/crates/toolchain/platform/src/memory.rs`).
     2. Passes `Elf` to the RV32 transpiler to obtain a `VmExe`.
   - Write unit tests (conditionally `ignore` if guest binaries absent) to load
     `guests/playground` ELF and assert non-empty program/memory.

6. **CLI command integration**
   - Update `src/main.rs` (or CLI module) to add Clap subcommand `RunElf`.
   - Handler resolves the path, invokes `runner::load_vmexe_from_elf`, prints
     summary (instruction count, pc_start).
   - Ensure `cargo run -- run-elf --path ./guests/playground/...` works.

7. **Documentation**
   - Update `README.md` (or create docs page) describing:
     - How to build the guest
       (`cargo +nightly build --target riscv32im-risc0-zkvm-elf -p guests-playground`).
     - How to use `cargo run -- run-elf --path ...`.
     - Mention OpenVM sources used for the implementation.

8. **Validation**
   - Rebuild guest ELF (from this repo’s `guests/playground`).
   - Run new CLI command; confirm `VmExe` creation succeeds (log size, optional
     serialization).
   - If needed, add integration test or script to assert parser works.

Deliverables: CLI command, runner APIs, simplified `VmExe` pipeline,
documentation, and tests. Once this plan is executed, the new codex instance
should follow it precisely, citing OpenVM sources where code was adapted.
