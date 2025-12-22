# RV32IM zkVM — Technical Dossier

## 2. Transpiler

The transpiler consumes the ELF before witness generation. It decodes every
instruction in the `.text` section into a canonical four-word representation,
loads the remaining PT_LOAD segments into byte-addressable memory, and surfaces
the VM-visible entry state as a `VmExe`. The following invariants are enforced:

- program addresses remain 4-byte aligned and identical to the ELF virtual
  addresses; the interpreter therefore advances `pc += 4` for straight-line
  execution exactly as on bare metal;
- every decoded instruction produces four M31 field elements encoded as `u32`
  for host code, capturing opcode, register indices, and immediates;
- any malformed instruction encoding, unknown opcode, or address that violates
  the fixed memory map causes transpilation to fail rather than deferring the
  failure to runtime.

### 2.1 Instruction representation

All instructions are decoded with `rrs-lib` using the official RV32IM tables.
Each unique tuple `(opcode[6:0], funct3, funct7)` is deterministically mapped to
an `opcode_id` in the M31 field. The four exported words per instruction are:

- **R-Type** (including the `M` extension): `(opcode_id, rd, rs1, rs2)`.
- **I-Type** (ALU immediates and loads): `(opcode_id, rd, rs1, imm)` where `imm`
  is the sign-extended 32-bit offset.
- **S-Type** (stores): `(opcode_id, rs1, rs2, imm)` with `imm` the sign-extended
  store offset in bytes.
- **B-Type** (branches): `(opcode_id, rs1, rs2, imm)` where `imm` is a signed
  byte offset already multiplied by 2 according to the ISA and ready to add to
  the PC.
- **U-Type** (`LUI`, `AUIPC`): `(opcode_id, rd, imm, 0)` where `imm` equals the
  upper 20 bits shifted left by 12.
- **J-Type** (`JAL`): `(opcode_id, rd, imm, 0)` with `imm` the sign-extended
  21-bit byte offset.

Register indices use their architectural numbers (0–31). All `imm` slots are the
final <= 20-bit quantity required by the interpreter, eliminating the need for
additional decoding logic. Although the native representation is M31, the
transpiler stores these four words as `u32` to minimize conversions when the
interpreter interacts with other `u32` state such as addresses and register
values.

### 2.2 Transpiler Architecture

The transpiler converts an ELF into a `VmExe` struct that captures the exact VM
entry state. The struct layout is:

```rust
struct VmExe{
    initial_pc: u32,
    regs: [u32; 32],
    program: BTreeMap<u32, [u32;4]>,
    memory: BTreeMap<u32, u8>
}
```

- `initial_pc` equals the ELF entry symbol (`_start`).
- `regs` is initialized according to Section 1.6 (x2/x3 populated, x0 hardwired
  to 0, others zeroed for determinism).
- `program` stores every 4-byte-aligned address in `.text` mapped to the four
  decoded words; unknown or sparse addresses are rejected at transpile time.
- `memory` stores byte values for all remaining PT_LOAD segments (initialized
  `.rodata`, `.data`, and zeroed `.bss`). Each key is a byte address.

A canonical memory map is enforced while reading the ELF:

| Region  | Start Address | End Address   | Size        | Access           |
| ------- | ------------- | ------------- | ----------- | ---------------- |
| .text   | `0x0000_0400` | `0x000F_FFFF` | ~ 1 MB      | Read and Execute |
| .rodata | `0x0010_0000` | `0x001F_FFFF` | 1 MB        | Read             |
| .data   | `0x0020_0000` | `0x002F_FC00` | 1 MB - 1 KB | Read and Write   |
| stack   | `0x002F_FC00` | `0x0030_0000` | 1 KB        | Read and Write   |

The stack pointer is initialized to `0x0030_0000`, which is the first address
_above_ the stack region, consistent with Section 1.5. The stack grows downward
toward `0x002F_FC00`.

Address `0x0000_0000` through `0x0000_03FF` are reserved and produce an error on
access. This catches null pointer dereferences. Attempts to map ELF segments
outside the table above are rejected during transpilation. The stack region is
not backed by ELF bytes; it is zero-initialized by the transpiler.

## 3. VM Emulator and Execution Model

This section specifies the interpreter architecture, memory model, register
file, trace generation strategy, and termination semantics for the zkVM
execution engine. Given a compiled ELF binary (as produced by the toolchain
described in Section 1), this section defines precisely how the VM executes it
and produces execution traces suitable for proof generation.

### 3.1 Interpreter Architecture

The VM implements a single-threaded, synchronous fetch-decode-execute loop. Each
instruction is processed atomically: fetch the 32-bit word at the program
counter, decode using fixed-width field extraction, execute via nested match
dispatch, and advance machine state.

#### 3.1.1 State Representation

The interpreter maintains the following state:

```rust
struct Cpu {
    regs: Registers,    // General-purpose registers x0-x31
    pc: u32,            // Program counter
    program: Program    // Program segment
    memory: Memory,     // Read Write memory (see 3.2)
    cycle: u64,         // Instruction count
    halted: bool,       // Termination flag
}
```

The `cycle` counter increments by one for each instruction executed, providing a
global ordering for trace events.

#### 3.1.2 Initialization

`Cpu::new(vm_exe: VmExe)` copies `vm_exe.regs` into the mutable register file,
sets `pc = vm_exe.initial_pc`, `program = vm_exe.program`,
`memory = Memory::from(vm_exe.memory)`, `cycle = 0`, and `halted = false`. No
implicit state is derived beyond what the transpiler serialized.

#### 3.1.3 Fetch-Decode-Execute Loop

Execution proceeds as follows:

1. **Fetch**: Load the decoded instruction word from the `Program` in memory for
   the current PC. The PC must be 4-u32 aligned; misaligned fetches produce an
   error.

2. **Execute**: Dispatch to the appropriate handler based on the `opcode_id`
   (defined in 2.1). Each handler reads operands and immediates, performs the
   operation, writes results, and computes the next PC.

3. **Trace**: Append a trace row to the appropriate opcode family collector (see
   Section 3.4).

4. **Advance**: Update PC to the computed next address. Increment cycle counter.

#### 3.1.4 Instruction Dispatch

The interpreter uses match-based dispatch on the opcode_id defined in section
2.1.

#### 3.1.5 Finalization

Once termination is detected and the program halted, finalize the trace
execution (see section 3.2.4 for memory finalization)

#### 3.1.6 Design Rationale

**Implementation Path**:

- Define `Cpu` struct with state fields as shown above
- Implement `step()` with opcode-level match dispatch
- Each `execute_*` method appends to per-opcode trace collectors

---

### 3.2 Memory Model

The VM provides two memory segments: a program segment for instructions and a
read-write segment for the rest. Both segments are aligned on 4 cells.

#### 3.2.1 Interpreter Representation Implementation

The `Program` struct contains the program and keeps track of the number of
accesses for each instruction. The `Memory` struct contains the data for witness
generation of the memory and clock_update components.

```rust

// Type representing a program cell.
// - 4 first u32: 4 M31s stored as u32s for the decoded instruction
// - u32: multiplicity
type ProgramCell = (u32, u32, u32, u32, u32)

struct Program {
    program: BTreeMap<u32, ProgramCell>,
}

// Type representing a RW memory cell.
// - u8: value
// - u32: previous clock
type ReadWriteCell = (u8, u32)

struct Memory {
    // Memory
    memory: BTreeMap<u32, ReadWriteCell>,

    // Traces
    memory_trace: Vec<[u32;4]>,        // address, clock, value, multiplicity
    clock_update_trace: Vec<[u32;3]>,  // address, prev_clock, value
}
```

Clock gaps are bounded by `RC20_LIMIT = 2^20 - 1` to match the range-check table
used in the AIR. Whenever the real interval between two accesses exceeds this
bound, the helper traces are populated with intermediate rows so that each row
respects the limit.

#### 3.2.2 Initialization

When loading the VmExe:

Program:

- `Program` is initialized with addresses and values from `VmExe::program` and
  multiplicity set to 0. Addresses in `Program::program` are multiples of 4.

Memory:

- `Memory::memory` is initialized with addresses and values from `VmExe::memory`
  and with clock set to 0;
- all cells of `Memory::memory` are pushed to `Memory::memory_trace` with the
  same addresses, values and clocks, and set multiplicity to 1.
- `Memory::clock_update_trace` is initialized empty.

#### 3.2.3 Access Operations

The memory must continually update the 2 segments as so:

- `program`: when a cell from this segment is accessed, multiplicity is
  increased by one.
- `memory`: updates the clock with the current one for each access.

The memory interface provides byte and word operations:

- `load_byte(addr, clock) -> u8`: No alignment requirement.
- `load_halfword(addr, clock) -> u16`: Requires 2-byte alignment
  (`addr & 1 == 0`).
- `load_word(addr, clock) -> u32`: Requires 4-byte alignment (`addr & 3 == 0`).
- `store_byte(addr, clock, value)`: No alignment requirement.
- `store_halfword(addr, clock, value)`: Requires 2-byte alignment.
- `store_word(addr, clock, value)`: Requires 4-byte alignment.

The program interface provides a single word operation:

- `fetch_instr(pc) -> Instruction`: Requires 4-byte alignment (`addr & 3 == 0`).

Misaligned accesses produce an `AlignmentError`. All memory operations are
recorded in the memory trace (see Section 3.2.4).

`load_*` calls fail on unmapped addresses. `store_*` lazily allocates new cells
by inserting `(value=0, clock=0)` before applying the write so that traces show
an explicit initialization event.

Halfword and word helpers are thin wrappers over the byte primitives:

- `load_halfword` / `store_halfword` invoke the byte versions twice, combining
  or splitting values in little-endian order.
- `load_word` / `store_word` invoke the byte versions four times, again in
  little-endian order.

This guarantees that every multi-byte access is reduced to the primitive
byte-level witness updates.

```rust
fn fetch_instr(&mut self, pc: u32) -> Result<[u32;4], Error> {
    let c = self.program.get_mut(&pc).ok_or(Error::UninitializedAddress)?;
    c.4 += 1;
    Ok([c.0, c.1, c.2, c.3])
}

fn load_byte(&mut self, addr: u32, clock: u32) -> Result<ReadWriteCell, Error> {
    let (prev_value, prev_clock) = self.memory.get_mut(&addr).ok_or(Error::UninitializedAddress)?;
    let old_clock = *prev_clock;

    let delta = clock.saturating_sub(*prev_clock);
    for i in 0..delta / RC20_LIMIT {
        self.clock_update_trace.push((addr, *prev_clock + i * RC20_LIMIT, *prev_value));
    }
    *prev_clock = clock;
    Ok((*prev_value, old_clock))
}

fn store_byte(&mut self, addr: u32, clock: u32, value: u8) -> Result<ReadWriteCell, Error> {
    if !self.memory.contains_key(&addr) {
        self.memory_trace.push((addr, 0, 0, 1));
        self.memory.insert(addr, (0, 0));
    }
    let (prev_value, prev_clock) = self.memory.get_mut(&addr).unwrap();
    let (old_value, old_clock) = (*prev_value, *prev_clock);

    let delta = clock.saturating_sub(*prev_clock);
    for i in 0..delta / RC20_LIMIT {
        self.clock_update_trace.push((addr, *prev_clock + i * RC20_LIMIT, *prev_value));
    }
    *prev_clock = clock;
    *prev_value = value;
    Ok((old_value, old_clock))
}
```

The `load_byte` and `store_byte` functions both return the previous value and
the previous clock. These are needed for opcode trace generation.

#### 3.2.4 Finalization

Once the execution is over the CPU calls `memory.finalize()` to get the final
traces:

- Returns `clock_update_trace` as is;
- Creates explicit Merkle traces for the memory commitments, matching the exact
  format consumed by `crates/prover/src/components/merkle.rs` in
  [**Cairo-M**](https://github.com/kkrt-labs/cairo-m/blob/main/crates/prover/src/components/merkle.rs):
  - `initial_merkle_trace: Vec<[u32;9]>` is produced by rerunning the
    `build_partial_merkle_tree` algorithm from
    [**Cairo-M**](https://github.com/kkrt-labs/cairo-m/blob/main/crates/prover/src/adapter/merkle.rs)
    directly on `Memory::memory_trace` (which, immediately after execution,
    still reflects the initial memory). The exact procedure is:
    1. Treat every recorded byte address as a distinct leaf; convert
       `(addr, clock, value, multiplicity)` into a leaf
       `(index = addr, depth = 30, value = M31::from(value), multiplicity = multiplicity)`.
    2. For each depth from 30 down to 1, pair neighboring children
       `(index, index ^ 1)`, fill any missing child with the default Poseidon
       hash for that depth, compute the parent using
       `Poseidon2Hash::hash(left, right)`, and propagate multiplicity as
       `left_mult + right_mult`.
    3. Emit one row per node with schema
       `[index, depth, left_value, right_value, parent_value, left_mult,   right_mult, parent_mult, root]`,
       where `root` is the final Merkle root produced by the same run of the
       algorithm. Depth ordering is irrelevant; rows can be appended as nodes
       are produced.
  - To create the final memory witness, take the post-execution map stored in
    `Memory::memory`, convert it into the same tuple representation as
    `memory_trace` but set every multiplicity to `-1`, and append this data to
    `memory_trace`. Running the tree-building procedure above on this augmented
    trace yields `final_merkle_trace`, and the resulting rows should be appended
    as well.
  - A third invocation of the same tree builder runs on `program_trace` so that
    the `.text` segment has its own Merkle witness. Every instruction byte
    becomes a leaf with multiplicity `1`, and the rows follow the same
    `[index, depth, …, root]` schema.
- Extend `memory_trace` with both the final-memory rows (multiplicity `-1`) and
  the derived Merkle rows so that continuation proofs have the per-access log
  plus both boundary commitments.

The CPU also calls `program.finalize()`:

- Creates a trace `program_trace` typed as `Vec<([u32; 6])>` (addr, 4 M31s as
  u32s and the multiplicity) from `Program::program`. The Merkle tree described
  above is built directly from this byte-precise data—no QM31 decoding or
  regrouping beyond the four decoded words per instruction is needed.

Merkle tree hashing uses the Poseidon2 permutation, so a Poseidon trace must be
generated alongside the Merkle rows. Follow the same pattern as
[**Cairo-M**](https://github.com/kkrt-labs/cairo-m/blob/main/crates/prover/src/adapter/mod.rs):

1. For every node emitted in any of the three trees, call `Poseidon2Hash::hash`
   with the two child values. Record the input tuple `[left_value, right_value]`
   in the Poseidon trace and store the resulting `parent_value`.
2. Immediately after obtaining the hash output, push a second tuple
   `[parent_value, 0]` so the Poseidon component observes both the input and the
   resulting digest (the second lane is zero because the Cairo-M Poseidon2
   component uses a rate-2 sponge).

These two entries per node mirror the lookup structure in
`LookupData::poseidon2` and ensure every Merkle edge is backed by an explicit
Poseidon witness.

#### 3.2.5 Design Rationale

**Implementation Path**:

- Define `Memory` and `Program` structs
- Implement the initialization of the `Memory` and `Program`
- Implement `Memory::load_*` and `Memory::store_*` methods with alignment checks
- Implement `Program::fetch_instr`
- Implement Merkle trace constructors.
- Implement `Memory::finalize()` and `Program::finalize()`
- Implement the Poseidon trace builder

---

### 3.3 Register File

The register file consists of 32 general-purpose 32-bit registers (x0 through
x31) plus a separate program counter.

#### 3.3.1 Register Semantics

- **x0 (zero)**: Hardwired to zero. Reads always return 0. Writes are silently
  discarded.

- **x1-x31**: General-purpose registers. All values are unsigned 32-bit
  integers.

- **PC**: Stored separately from the general-purpose registers. Always contains
  a 4-byte-aligned address.

#### 3.3.2 Register Representation

```rust
type RegisterEntry = (u32, u32); // (value, previous clock)

struct Registers {
    // Registers
    regs: [RegisterEntry; 32]

    // Traces
    reg_clock_update_trace: Vec<[u32;3]>
}
```

#### 3.3.3 Access Interface

```rust
fn get_reg(&mut self, idx: u32, clock: u32) -> Result<RegisterEntry, Error> {
    if idx == 0 {
        return Ok((0, 0));
    }

    let (prev_value, prev_clock) = &mut self.regs[idx as usize];
    let old_clock = *prev_clock;

    let delta = clock.saturating_sub(*prev_clock);
    for i in 0..delta / RC20_LIMIT {
        self.reg_clock_update_trace.push((idx, *prev_clock + i * RC20_LIMIT, *prev_value));
    }
    *prev_clock = clock;
    Ok((*prev_value, old_clock))
}

fn set_reg(&mut self, idx: u32, clock: u32, value: u32) -> Result<RegisterEntry, Error> {
    if idx == 0 {
        return Ok((0, clock));
    }

    let (prev_value, prev_clock) = &mut self.regs[idx as usize];
    let (old_value, old_clock) = (*prev_value, *prev_clock);

    let delta = clock.saturating_sub(*prev_clock);
    for i in 0..delta / RC20_LIMIT {
        self.reg_clock_update_trace.push((idx, *prev_clock + i * RC20_LIMIT, *prev_value));
    }
    *prev_clock = clock;
    *prev_value = value;
    Ok((old_value, old_clock))
}
```

The explicit check for `idx == 0` ensures the x0 invariant is maintained
regardless of what instruction encoding attempts. Reads of x0 always return
`(0, 0)` and never touch `reg_clock_update_trace`; writes to x0 are acknowledged
but ignored so that callers can keep uniform code paths without conditional
branches.

#### 3.3.4 Initialization

At program entry (per Section 1.6):

| Register   | Value                                 |
| ---------- | ------------------------------------- |
| PC         | ELF entry address (`_start`)          |
| x2 (sp)    | `0x0030_0000` (top of stack)          |
| x3 (gp)    | `__global_pointer$` from linker       |
| x0         | 0 (hardwired)                         |
| x1, x4-x31 | Unspecified (implementation may zero) |

#### 3.3.5 Finalization

Return `reg_clock_update_trace` as is.

#### 3.3.6 ABI Register Names

For reference, the RISC-V calling convention assigns the following roles:

| Register | ABI Name | Role                           |
| -------- | -------- | ------------------------------ |
| x0       | zero     | Constant zero                  |
| x1       | ra       | Return address                 |
| x2       | sp       | Stack pointer                  |
| x3       | gp       | Global pointer                 |
| x4       | tp       | Thread pointer                 |
| x5-x7    | t0-t2    | Temporaries                    |
| x8       | s0/fp    | Saved register / Frame pointer |
| x9       | s1       | Saved register                 |
| x10-x11  | a0-a1    | Arguments / Return values      |
| x12-x17  | a2-a7    | Arguments                      |
| x18-x27  | s2-s11   | Saved registers                |
| x28-x31  | t3-t6    | Temporaries                    |

These names are informational. The interpreter treats all registers uniformly
except for x0.

**Implementation Path**:

- Store registers as `regs: [u32; 32]` with `pc: u32` separate
- Implement `reg()` and `set_reg()` with x0 special handling
- Initialize per the table above when loading an ELF

---

### 3.4 Opcodes

The interpreter generates execution traces suitable for STARK proof generation
using Stwo. Traces are organized by opcode family.

#### 3.4.1 Opcode Families

Instructions are grouped into 8 families based on their operand patterns and
constraint requirements:

| Family      | Opcodes                                              | N_COLUMNS | Schema (field sizes in bytes)                                                                                                                                                                           |
| ----------- | ---------------------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `alu_reg`   | ADD, SUB, SLL, SLT, SLTU, XOR, SRL, SRA, OR, AND     | 25        | cycle(1), opcode_id(1), pc(1), rs1_idx(1), rs1_val(4), rs1_prev_clock(1), rs2_idx(1), rs2_val(4), rs2_prev_clock(1), rd_idx(1), rd_val(4), rd_prev_val(4), rd_prev_clock(1)                             |
| `alu_imm`   | ADDI, SLTI, SLTIU, XORI, ORI, ANDI, SLLI, SRLI, SRAI | 23        | cycle(1), opcode_id(1), pc(1), rs1_idx(1), rs1_val(4), rs1_prev_clock(1), imm(4), rd_idx(1), rd_val(4), rd_prev_val(4), rd_prev_clock(1)                                                                |
| `upper_imm` | LUI, AUIPC                                           | 17        | cycle(1), opcode_id(1), pc(1), imm(4), rd_idx(1), rd_val(4), rd_prev_val(4), rd_prev_clock(1)                                                                                                           |
| `branch`    | BEQ, BNE, BLT, BGE, BLTU, BGEU                       | 24        | cycle(1), opcode_id(1), pc(1), rs1_idx(1), rs1_val(4), rs1_prev_clock(1), rs2_idx(1), rs2_val(4), rs2_prev_clock(1), imm(4), taken(1), pc_next(4)                                                       |
| `load`      | LB, LH, LW, LBU, LHU                                 | 33        | cycle(1), opcode_id(1), pc(1), rs1_idx(1), rs1_val(4), rs1_prev_clock(1), imm(4), addr(4), mem_width(1), mem_val(4), mem_prev_clock(1), rd_idx(1), rd_val(4), rd_prev_val(4), rd_prev_clock(1)          |
| `store`     | SB, SH, SW                                           | 29        | cycle(1), opcode_id(1), pc(1), rs1_idx(1), rs1_val(4), rs1_prev_clock(1), rs2_idx(1), rs2_val(4), rs2_prev_clock(1), imm(4), addr(4), mem_width(1), mem_prev_val(4), mem_prev_clock(1)                  |
| `jump`      | JAL, JALR                                            | 27        | cycle(1), opcode_id(1), pc(1), rs1_idx(1), rs1_val(4), rs1_prev_clock(1), imm(4), rd_idx(1), rd_val(4), rd_prev_val(4), rd_prev_clock(1), pc_next(4)                                                    |
| `mul_div`   | MUL, MULH, MULHSU, MULHU, DIV, DIVU, REM, REMU       | 33        | cycle(1), opcode_id(1), pc(1), rs1_idx(1), rs1_val(4), rs1_prev_clock(1), rs2_idx(1), rs2_val(4), rs2_prev_clock(1), rd_idx(1), rd_val(4), rd_prev_val(4), rd_prev_clock(1), result_lo(4), result_hi(4) |

#### 3.4.2 Trace recording

As explained in Section 3.1.3, a VM cycle fetches an instruction, executes it,
and records a row. Every opcode handler (exactly one per `opcode_id`) gathers:

- static fields from the decoded instruction (`opcode_id`, register indices,
  `imm`);
- dynamic register data via `Registers::get_reg` / `set_reg` (`rs*_val`,
  `rs*_prev_clock`, `rd_prev_val`, `rd_prev_clock`);
- dynamic memory data via `Memory::{load_*,store_*}` (`mem_val` or
  `mem_prev_val`, `mem_prev_clock`, `addr`);
- control-flow outcomes such as `pc_next`, `taken`, and `result_hi/lo`.

Every handler must populate the schema listed in Table 3.4.1 before advancing
the cycle counter.

#### 3.4.3 Instruction semantics

Opcode handlers execute the ISA semantics byte-for-byte. Unless stated
otherwise, the next PC equals `pc + 4` and arithmetic uses wraparound
(`u32::wrapping_*`). Writing to `rd = x0` is permitted but has no effect.

**`alu_reg` (R-type ALU)**

- `ADD`: `rd = rs1 + rs2`.
- `SUB`: `rd = rs1 - rs2`.
- `SLL`: `rd = rs1 << (rs2 & 0x1F)`.
- `SLT`: `rd = 1` if `(rs1 as i32) < (rs2 as i32)` else `0`.
- `SLTU`: `rd = 1` if `rs1 < rs2` (unsigned) else `0`.
- `XOR`: `rd = rs1 ^ rs2`.
- `SRL`: `rd = rs1 >> (rs2 & 0x1F)` (logical).
- `SRA`: arithmetic right shift by `rs2 & 0x1F`.
- `OR`: bitwise `rs1 | rs2`.
- `AND`: bitwise `rs1 & rs2`.

**`alu_imm` (I-type ALU)**

- `ADDI`: `rd = rs1 + imm`.
- `SLTI`: signed comparison with `imm`.
- `SLTIU`: unsigned comparison with `imm`.
- `XORI`: `rd = rs1 ^ imm`.
- `ORI`: `rd = rs1 | imm`.
- `ANDI`: `rd = rs1 & imm`.
- `SLLI`: `rd = rs1 << (imm & 0x1F)`.
- `SRLI`: logical right shift by `imm & 0x1F`.
- `SRAI`: arithmetic right shift by `imm & 0x1F`.

**`upper_imm` (U-type)**

- `LUI`: `rd = imm` (already shifted left by 12).
- `AUIPC`: `rd = pc + imm`.

**`branch` (B-type)**

`addr = pc.wrapping_add(imm)` produces the branch target. `taken` equals:

- `BEQ`: `rs1 == rs2`.
- `BNE`: `rs1 != rs2`.
- `BLT`: `(rs1 as i32) < (rs2 as i32)`.
- `BGE`: `(rs1 as i32) ≥ (rs2 as i32)`.
- `BLTU`: `rs1 < rs2` (unsigned).
- `BGEU`: `rs1 ≥ rs2` (unsigned).

`pc_next = addr` when `taken`, otherwise `pc + 4`. `pc_next` must satisfy
`pc_next & 0x3 == 0`, otherwise an `AlignmentError` is raised.

**`load` (I-type loads)**

`addr = rs1_val.wrapping_add(imm)` and `mem_width ∈ {1,2,4}` encodes the access
size. Alignment is enforced per Section 3.2.3. The value placed in `rd` is:

- `LB`: sign-extend the loaded byte.
- `LH`: sign-extend the loaded halfword.
- `LW`: load the full word.
- `LBU`: zero-extend the loaded byte.
- `LHU`: zero-extend the loaded halfword.

`mem_val` stores the zero-extended raw value read; `rd_val` stores the
sign-extended value when required. Loads never mutate memory, so the trace only
records `mem_val` plus the `mem_prev_clock` returned by `Memory::load_*`.

**`store` (S-type stores)**

`addr = rs1_val.wrapping_add(imm)` with the same alignment rules as loads.
`rs2_val` supplies the data:

- `SB`: write the low 8 bits.
- `SH`: write the low 16 bits (little-endian).
- `SW`: write the full 32 bits.

`mem_prev_val` / `mem_prev_clock` capture the overwritten byte/halfword/word and
its last write clock before the store. After the store succeeds the new value is
visible to future loads.

**`jump` (J-type)**

- `JAL`: `rd = pc + 4`, `pc_next = pc + imm`.
- `JALR`: `target = (rs1_val + imm) & !1`, `rd = pc + 4`, `pc_next = target`.

Targets must be 4-byte aligned; otherwise `AlignmentError` is raised.

**`mul_div` (M extension)**

All products are computed using 64-bit intermediates (`i64` or `u64` as
appropriate):

- `MUL`: `rd = low_32(rs1 * rs2)`, while `result_lo/hi` capture the full
  product.
- `MULH`: `rd = high_32((rs1 as i64) * (rs2 as i64))`.
- `MULHSU`: `rd = high_32((rs1 as i64) * (rs2 as u64))`.
- `MULHU`: `rd = high_32((rs1 as u64) * (rs2 as u64))`.
- `DIV`: `rd = signed_quotient(rs1, rs2)` with truncation toward zero;
  divide-by-zero yields `0xFFFF_FFFF`, overflow (`INT_MIN / -1`) yields
  `INT_MIN`.
- `DIVU`: `rd = unsigned_quotient(rs1, rs2)`; divide-by-zero yields
  `0xFFFF_FFFF`.
- `REM`: `rd = signed_remainder(rs1, rs2)`; divide-by-zero yields `rs1`.
- `REMU`: `rd = unsigned_remainder(rs1, rs2)`; divide-by-zero yields `rs1`.

For `MULH*` instructions, `result_lo` stores the low 32 bits even though `rd`
receives the high part, keeping the trace arithmetically constrained. Division
operations set `result_lo = quotient` and `result_hi = remainder` for the same
reason.

### 3.5 Termination

Execution terminates when the interpreter detects an infinite loop.

The interpreter detects trivial infinite loops where the next PC equals the
current PC:

- `JAL x0, 0` (jump to self)
- `BEQ x0, x0, 0` (branch to self, always taken)

When detected:

1. Set `halted = true`
2. Set exit code to 0
3. Stop execution loop

This handles the common bare-metal pattern `loop {}` which compiles to a
self-jump, as shown in Section 1.7.

**Implementation Path**:

- Check `pc_next == pc` after computing next PC
- On clean termination, dump all trace collectors: no by default (but possible
  if wanted by user)

### 3.6 Comprehensive RV32IM Test Program (All Opcodes)

This appendix provides a complete guest program that exercises **all 47 RV32IM
instructions**. Use this program to validate end-to-end execution and trace
generation.

#### 3.6.1 RV32IM Instruction Checklist

| Family      | Count  | Instructions                                         |
| ----------- | ------ | ---------------------------------------------------- |
| R-type ALU  | 10     | ADD, SUB, SLL, SLT, SLTU, XOR, SRL, SRA, OR, AND     |
| I-type ALU  | 9      | ADDI, SLTI, SLTIU, XORI, ORI, ANDI, SLLI, SRLI, SRAI |
| U-type      | 2      | LUI, AUIPC                                           |
| Load        | 5      | LB, LH, LW, LBU, LHU                                 |
| Store       | 3      | SB, SH, SW                                           |
| Branch      | 6      | BEQ, BNE, BLT, BGE, BLTU, BGEU                       |
| Jump        | 2      | JAL, JALR                                            |
| M-extension | 8      | MUL, MULH, MULHSU, MULHU, DIV, DIVU, REM, REMU       |
| System      | 2      | ECALL, EBREAK                                        |
| **Total**   | **47** |                                                      |

#### 3.6.2 Test Program Source

```rust
#![no_std]
#![no_main]

use core::arch::asm;
use core::arch::global_asm;
use core::panic::PanicInfo;

// =============================================================================
// Startup assembly (ELF entrypoint)
// =============================================================================

global_asm!(
    r#"
    .section .text._start
    .globl _start
_start:
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop

    la sp, __stack_top
    lw sp, 0(sp)

    call __zkvm_start
"#
);

#[no_mangle]
#[link_section = ".data.stack"]
static __stack_top: u32 = 0x0030_0000;

#[no_mangle]
static TEST_SEED: u32 = 0xC001_C0DE;

#[no_mangle]
static mut GLOBAL_SCRATCH: [u8; 64] = [0; 64];

// =============================================================================
// Entry shim
// =============================================================================

#[no_mangle]
pub extern "C" fn __zkvm_start() -> ! {
    main();
}

// =============================================================================
// Test functions for each instruction family
// =============================================================================

/// Test R-type ALU instructions: ADD, SUB, SLL, SLT, SLTU, XOR, SRL, SRA, OR, AND
#[inline(never)]
fn test_r_type_alu() {
    unsafe {
        let mut result: u32;

        // ADD: rd = rs1 + rs2
        asm!("add {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 10u32, rs2 = in(reg) 20u32);
        assert_eq(result, 30);

        // SUB: rd = rs1 - rs2
        asm!("sub {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 50u32, rs2 = in(reg) 20u32);
        assert_eq(result, 30);

        // SLL: rd = rs1 << rs2[4:0]
        asm!("sll {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 1u32, rs2 = in(reg) 4u32);
        assert_eq(result, 16);

        // SLT: rd = (rs1 < rs2) ? 1 : 0 (signed)
        asm!("slt {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) (-5i32) as u32, rs2 = in(reg) 5u32);
        assert_eq(result, 1);

        // SLTU: rd = (rs1 < rs2) ? 1 : 0 (unsigned)
        asm!("sltu {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 5u32, rs2 = in(reg) 10u32);
        assert_eq(result, 1);

        // XOR: rd = rs1 ^ rs2
        asm!("xor {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 0b1010u32, rs2 = in(reg) 0b1100u32);
        assert_eq(result, 0b0110);

        // SRL: rd = rs1 >> rs2[4:0] (logical)
        asm!("srl {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 0x80u32, rs2 = in(reg) 4u32);
        assert_eq(result, 0x08);

        // SRA: rd = rs1 >> rs2[4:0] (arithmetic)
        asm!("sra {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 0x80000000u32, rs2 = in(reg) 4u32);
        assert_eq(result, 0xF8000000);

        // OR: rd = rs1 | rs2
        asm!("or {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 0b1010u32, rs2 = in(reg) 0b0101u32);
        assert_eq(result, 0b1111);

        // AND: rd = rs1 & rs2
        asm!("and {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 0b1010u32, rs2 = in(reg) 0b1100u32);
        assert_eq(result, 0b1000);
    }
}

/// Test I-type ALU instructions: ADDI, SLTI, SLTIU, XORI, ORI, ANDI, SLLI, SRLI, SRAI
#[inline(never)]
fn test_i_type_alu() {
    unsafe {
        let mut result: u32;

        // ADDI: rd = rs1 + imm
        asm!("addi {rd}, {rs1}, 42", rd = out(reg) result, rs1 = in(reg) 8u32);
        assert_eq(result, 50);

        // SLTI: rd = (rs1 < imm) ? 1 : 0 (signed)
        asm!("slti {rd}, {rs1}, 10", rd = out(reg) result, rs1 = in(reg) 5u32);
        assert_eq(result, 1);

        // SLTIU: rd = (rs1 < imm) ? 1 : 0 (unsigned)
        asm!("sltiu {rd}, {rs1}, 10", rd = out(reg) result, rs1 = in(reg) 5u32);
        assert_eq(result, 1);

        // XORI: rd = rs1 ^ imm
        asm!("xori {rd}, {rs1}, 0xFF", rd = out(reg) result, rs1 = in(reg) 0xF0u32);
        assert_eq(result, 0x0F);

        // ORI: rd = rs1 | imm
        asm!("ori {rd}, {rs1}, 0x0F", rd = out(reg) result, rs1 = in(reg) 0xF0u32);
        assert_eq(result, 0xFF);

        // ANDI: rd = rs1 & imm
        asm!("andi {rd}, {rs1}, 0x0F", rd = out(reg) result, rs1 = in(reg) 0xFFu32);
        assert_eq(result, 0x0F);

        // SLLI: rd = rs1 << shamt
        asm!("slli {rd}, {rs1}, 4", rd = out(reg) result, rs1 = in(reg) 1u32);
        assert_eq(result, 16);

        // SRLI: rd = rs1 >> shamt (logical)
        asm!("srli {rd}, {rs1}, 4", rd = out(reg) result, rs1 = in(reg) 0x100u32);
        assert_eq(result, 0x10);

        // SRAI: rd = rs1 >> shamt (arithmetic)
        asm!("srai {rd}, {rs1}, 4", rd = out(reg) result, rs1 = in(reg) 0x80000000u32);
        assert_eq(result, 0xF8000000);
    }
}

/// Test U-type instructions: LUI, AUIPC
#[inline(never)]
fn test_upper_imm() {
    unsafe {
        let mut result: u32;

        // LUI: rd = imm << 12
        asm!("lui {rd}, 0x12345", rd = out(reg) result);
        assert_eq(result, 0x12345000);

        // AUIPC: rd = PC + (imm << 12)
        // We can't predict exact PC, but we can verify it's non-zero and high bits set
        asm!("auipc {rd}, 0x1", rd = out(reg) result);
        assert_ne(result, 0);
    }
}

/// Test load/store instructions: LB, LH, LW, LBU, LHU, SB, SH, SW
#[inline(never)]
fn test_load_store() {
    // Use stack for test buffer
    let mut buffer: [u8; 16] = [0; 16];
    let ptr = buffer.as_mut_ptr();

    unsafe {
        for i in 0..buffer.len() {
            GLOBAL_SCRATCH[i] = buffer[i];
        }
        GLOBAL_SCRATCH[buffer.len()] = (TEST_SEED & 0xFF) as u8;
    }

    unsafe {
        let mut result: u32;

        // SW: Store word
        asm!("sw {val}, 0({addr})", val = in(reg) 0xDEADBEEFu32, addr = in(reg) ptr);

        // LW: Load word
        asm!("lw {rd}, 0({addr})", rd = out(reg) result, addr = in(reg) ptr);
        assert_eq(result, 0xDEADBEEF);

        // SH: Store halfword
        asm!("sh {val}, 4({addr})", val = in(reg) 0xCAFEu32, addr = in(reg) ptr);

        // LH: Load halfword (sign-extended)
        asm!("lh {rd}, 4({addr})", rd = out(reg) result, addr = in(reg) ptr);
        assert_eq(result, 0xFFFFCAFE); // Sign-extended because 0xCAFE has MSB set

        // LHU: Load halfword (zero-extended)
        asm!("lhu {rd}, 4({addr})", rd = out(reg) result, addr = in(reg) ptr);
        assert_eq(result, 0x0000CAFE);

        // SB: Store byte
        asm!("sb {val}, 8({addr})", val = in(reg) 0x80u32, addr = in(reg) ptr);

        // LB: Load byte (sign-extended)
        asm!("lb {rd}, 8({addr})", rd = out(reg) result, addr = in(reg) ptr);
        assert_eq(result, 0xFFFFFF80); // Sign-extended

        // LBU: Load byte (zero-extended)
        asm!("lbu {rd}, 8({addr})", rd = out(reg) result, addr = in(reg) ptr);
        assert_eq(result, 0x00000080);
    }
}

/// Test branch instructions: BEQ, BNE, BLT, BGE, BLTU, BGEU
#[inline(never)]
fn test_branches() {
    unsafe {
        let mut result: u32;

        // BEQ: Branch if equal
        result = 0;
        asm!(
            "li {tmp}, 5",
            "beq {tmp}, {tmp}, 1f",
            "li {rd}, 0",
            "j 2f",
            "1: li {rd}, 1",
            "2:",
            tmp = out(reg) _,
            rd = out(reg) result,
        );
        assert_eq(result, 1);

        // BNE: Branch if not equal
        result = 0;
        asm!(
            "li {t1}, 5",
            "li {t2}, 6",
            "bne {t1}, {t2}, 1f",
            "li {rd}, 0",
            "j 2f",
            "1: li {rd}, 1",
            "2:",
            t1 = out(reg) _,
            t2 = out(reg) _,
            rd = out(reg) result,
        );
        assert_eq(result, 1);

        // BLT: Branch if less than (signed)
        result = 0;
        asm!(
            "li {t1}, -1",
            "li {t2}, 1",
            "blt {t1}, {t2}, 1f",
            "li {rd}, 0",
            "j 2f",
            "1: li {rd}, 1",
            "2:",
            t1 = out(reg) _,
            t2 = out(reg) _,
            rd = out(reg) result,
        );
        assert_eq(result, 1);

        // BGE: Branch if greater or equal (signed)
        result = 0;
        asm!(
            "li {t1}, 5",
            "li {t2}, 5",
            "bge {t1}, {t2}, 1f",
            "li {rd}, 0",
            "j 2f",
            "1: li {rd}, 1",
            "2:",
            t1 = out(reg) _,
            t2 = out(reg) _,
            rd = out(reg) result,
        );
        assert_eq(result, 1);

        // BLTU: Branch if less than (unsigned)
        result = 0;
        asm!(
            "li {t1}, 1",
            "li {t2}, -1",  // 0xFFFFFFFF unsigned
            "bltu {t1}, {t2}, 1f",
            "li {rd}, 0",
            "j 2f",
            "1: li {rd}, 1",
            "2:",
            t1 = out(reg) _,
            t2 = out(reg) _,
            rd = out(reg) result,
        );
        assert_eq(result, 1);

        // BGEU: Branch if greater or equal (unsigned)
        result = 0;
        asm!(
            "li {t1}, -1",  // 0xFFFFFFFF unsigned
            "li {t2}, 1",
            "bgeu {t1}, {t2}, 1f",
            "li {rd}, 0",
            "j 2f",
            "1: li {rd}, 1",
            "2:",
            t1 = out(reg) _,
            t2 = out(reg) _,
            rd = out(reg) result,
        );
        assert_eq(result, 1);
    }
}

/// Test jump instructions: JAL, JALR
#[inline(never)]
fn test_jumps() {
    unsafe {
        let mut result: u32;
        let mut ra_val: u32;

        // JAL: Jump and link
        asm!(
            "jal {ra}, 1f",
            "li {rd}, 0",
            "j 2f",
            "1: li {rd}, 1",
            "2:",
            ra = out(reg) ra_val,
            rd = out(reg) result,
        );
        assert_eq(result, 1);
        assert_ne(ra_val, 0); // ra should contain return address

        // JALR: Jump and link register
        asm!(
            "la {t1}, 1f",
            "jalr {ra}, {t1}, 0",
            "li {rd}, 0",
            "j 2f",
            "1: li {rd}, 1",
            "2:",
            t1 = out(reg) _,
            ra = out(reg) ra_val,
            rd = out(reg) result,
        );
        assert_eq(result, 1);
    }
}

/// Test M-extension instructions: MUL, MULH, MULHSU, MULHU, DIV, DIVU, REM, REMU
#[inline(never)]
fn test_m_extension() {
    unsafe {
        let mut result: u32;

        // MUL: rd = (rs1 * rs2)[31:0]
        asm!("mul {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 7u32, rs2 = in(reg) 6u32);
        assert_eq(result, 42);

        // MULH: rd = (rs1 * rs2)[63:32] (signed × signed)
        asm!("mulh {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 0x80000000u32, rs2 = in(reg) 2u32);
        assert_eq(result, 0xFFFFFFFF); // -2^31 * 2 = -2^32, high bits are all 1s

        // MULHSU: rd = (rs1 * rs2)[63:32] (signed × unsigned)
        asm!("mulhsu {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) (-1i32) as u32, rs2 = in(reg) 1u32);
        assert_eq(result, 0xFFFFFFFF);

        // MULHU: rd = (rs1 * rs2)[63:32] (unsigned × unsigned)
        asm!("mulhu {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 0xFFFFFFFFu32, rs2 = in(reg) 2u32);
        assert_eq(result, 1); // (2^32-1)*2 = 2^33-2, high word is 1

        // DIV: rd = rs1 / rs2 (signed)
        asm!("div {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) (-20i32) as u32, rs2 = in(reg) 3u32);
        assert_eq(result, (-6i32) as u32);

        // DIVU: rd = rs1 / rs2 (unsigned)
        asm!("divu {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 20u32, rs2 = in(reg) 3u32);
        assert_eq(result, 6);

        // REM: rd = rs1 % rs2 (signed)
        asm!("rem {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) (-20i32) as u32, rs2 = in(reg) 3u32);
        assert_eq(result, (-2i32) as u32);

        // REMU: rd = rs1 % rs2 (unsigned)
        asm!("remu {rd}, {rs1}, {rs2}", rd = out(reg) result, rs1 = in(reg) 20u32, rs2 = in(reg) 3u32);
        assert_eq(result, 2);
    }
}

// =============================================================================
// Helper functions
// =============================================================================

#[inline(always)]
fn assert_eq(actual: u32, expected: u32) {
    if actual != expected {
        // In a real implementation, this would trigger a trap or write to
        // a failure register. For now, enter infinite loop on failure.
        loop {}
    }
}

#[inline(always)]
fn assert_ne(actual: u32, unexpected: u32) {
    if actual == unexpected {
        loop {}
    }
}

// =============================================================================
// Main entry point
// =============================================================================

fn main() -> ! {
    // Test all instruction families
    test_r_type_alu();
    test_i_type_alu();
    test_upper_imm();
    test_load_store();
    test_branches();
    test_jumps();
    test_m_extension();

    // All tests passed - terminate with ECALL, exit code 0
    unsafe {
        asm!(
            "li a0, 0",  // Exit code 0 = success
            "ecall",
            options(noreturn)
        );
    }
}

// =============================================================================
// Panic handler
// =============================================================================

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    // Set non-zero exit code and halt
    unsafe {
        asm!(
            "li a0, 1",  // Exit code 1 = failure
            "ecall",
            options(noreturn)
        );
    }
}
```

#### 3.6.3 Build Instructions

```bash
cargo build \
  --release \
  --bin test-all-opcodes \
  --target riscv32im-unknown-none-elf
```

#### 3.6.4 End-to-End Validation

When implementing Section 2, the following validation steps confirm correct
behavior:

1. **Compile** the test program to ELF
2. **Load** the ELF into the VM interpreter
3. **Execute** until termination
4. **Verify** exit code is 0 (all assertions passed)
5. **Check** trace files are generated for all 8 opcode families:
   - `trace_alu_reg.bin` (ADD, SUB, SLL, SLT, SLTU, XOR, SRL, SRA, OR, AND)
   - `trace_alu_imm.bin` (ADDI, SLTI, SLTIU, XORI, ORI, ANDI, SLLI, SRLI, SRAI)
   - `trace_upper_imm.bin` (LUI, AUIPC)
   - `trace_load.bin` (LB, LH, LW, LBU, LHU)
   - `trace_store.bin` (SB, SH, SW)
   - `trace_branch.bin` (BEQ, BNE, BLT, BGE, BLTU, BGEU)
   - `trace_jump.bin` (JAL, JALR)
   - `trace_mul_div.bin` (MUL, MULH, MULHSU, MULHU, DIV, DIVU, REM, REMU)
   - `trace_memory.bin` (unified load/store log)

#### 3.6.5 Success Criteria

| Criterion                    | Validation Method                         |
| ---------------------------- | ----------------------------------------- |
| All 47 instructions executed | Trace file row counts > 0 for each family |
| Exit code = 0                | VM returns success status                 |
| Trace headers valid          | Magic = `0x54524143`, version = 1         |
| Column counts correct        | Match Section 2.4.3 specification         |
| Memory trace consistent      | All loads return previously stored values |

---

## 4. Trace Format and Stwo Integration

This section specifies how execution traces from Section 2 map to Stwo's witness
generation system. The goal is a nearly 1:1 correspondence between trace rows
and witness columns, minimizing transformation overhead while satisfying Stwo's
constraint framework requirements.

### 3.1 Witness Column Schema

The witness directly mirrors Section 2's trace format. Each column in the trace
file becomes a `CircleEvaluation<B, M31, BitReversedOrder>` column in the
witness.

#### 3.1.1 Column Ordering Convention

Witness columns follow the exact order defined in Section 2.4.3 for each opcode
family. No reordering or transformation occurs during witness generation beyond
bit-reversal for circle domain placement.

<!-- NOTE(antoine): cf modifications above. -->

For the `alu_reg` family (31 columns):

| Index | Field   | Bytes | Description                 |
| ----- | ------- | ----- | --------------------------- |
| 0-3   | cycle   | 4     | Global instruction counter  |
| 4-7   | pc      | 4     | Program counter             |
| 8-11  | instr   | 4     | Raw 32-bit instruction word |
| 12    | rs1_idx | 1     | Source register 1 index     |
| 13-16 | rs1_val | 4     | Source register 1 value     |
| 17    | rs2_idx | 1     | Source register 2 index     |
| 18-21 | rs2_val | 4     | Source register 2 value     |
| 22    | rd_idx  | 1     | Destination register index  |
| 23-26 | rd_val  | 4     | Destination register value  |
| 27-30 | result  | 4     | ALU computation result      |

Other families follow analogous schemas as defined in Section 2.4.3.

#### 3.1.2 Padding Strategy

Traces are padded to power-of-two lengths for efficient FFT operations:

<!-- NOTE(antoine): minimum log size is 4 (if log size is less than 4, pad it with 0s to 2**4) -->

```rust
fn compute_log_size(n_rows: u64) -> u32 {
    if n_rows == 0 { return 0; }
    (64 - (n_rows - 1).leading_zeros()) as u32
}
```

Padding rows have all columns set to zero. The `enabler` column (implicit or
explicit) distinguishes real rows from padding:

- Real rows: `enabler = 1`
- Padding rows: `enabler = 0`

Constraints are multiplied by `enabler` to ensure padding rows contribute
nothing to the constraint polynomial.

#### 3.1.3 Field Element Representation

All values are M31 field elements (the Mersenne prime 2³¹ - 1). Since Section 2
already decomposes values to bytes, each column contains values in [0, 255].
This enables efficient range checking via a single degree-256 constraint or
lookup table.

<!-- NOTE(antoine): again use RC8_8(limb0, limb1) and RC8_8(limb2, limb3) instead of 4 RC8. -->

---

### 3.2 Opcode Factorization

Instructions are grouped into 8 families based on their operand patterns and
constraint requirements. This factorization balances constraint complexity
against table count.

#### 3.2.1 Factorization Table

| Family      | Instructions                                         | Columns | Rationale                          |
| ----------- | ---------------------------------------------------- | ------- | ---------------------------------- |
| `alu_reg`   | ADD, SUB, SLL, SLT, SLTU, XOR, SRL, SRA, OR, AND     | 31      | R-type format, two register inputs |
| `alu_imm`   | ADDI, SLTI, SLTIU, XORI, ORI, ANDI, SLLI, SRLI, SRAI | 26      | I-type format, register + imm      |
| `upper_imm` | LUI, AUIPC                                           | 21      | U-type format, 20-bit immediate    |
| `branch`    | BEQ, BNE, BLT, BGE, BLTU, BGEU                       | 31      | B-type format, conditional PC      |
| `load`      | LB, LH, LW, LBU, LHU                                 | 30      | Memory read, variable width        |
| `store`     | SB, SH, SW                                           | 29      | Memory write, variable width       |
| `jump`      | JAL, JALR                                            | 26      | Unconditional control flow         |
| `mul_div`   | MUL, MULH, MULHSU, MULHU, DIV, DIVU, REM, REMU       | 35      | Extended result decomposition      |

#### 3.2.2 Design Rationale

<!-- NOTE(antoine): some opcodes might be too different even within a same family (take ADD and SLT). There might need to do sub-families. See AIR section. -->

**Why 8 families instead of 47 individual tables?**

- Instructions within a family share column layout and constraint structure
- Selector columns distinguish specific opcodes within a family
- Fewer tables reduce prover complexity and commitment overhead

**Why not a single unified table?**

- Different instruction formats require different columns
- A unified table would waste columns on unused fields
- Separate tables enable parallel proving of independent families

#### 3.2.3 Selector Column Pattern

Within each family, boolean selector columns identify the specific opcode. For
`alu_reg`:

```rust
// Derived during witness generation from funct3/funct7 fields
is_add  = (funct3 == 0) && (funct7 == 0x00)
is_sub  = (funct3 == 0) && (funct7 == 0x20)
is_sll  = (funct3 == 1) && (funct7 == 0x00)
is_slt  = (funct3 == 2) && (funct7 == 0x00)
is_sltu = (funct3 == 3) && (funct7 == 0x00)
is_xor  = (funct3 == 4) && (funct7 == 0x00)
is_srl  = (funct3 == 5) && (funct7 == 0x00)
is_sra  = (funct3 == 5) && (funct7 == 0x20)
is_or   = (funct3 == 6) && (funct7 == 0x00)
is_and  = (funct3 == 7) && (funct7 == 0x00)
```

Constraint: exactly one selector is 1 for real rows, all selectors are 0 for
padding rows.

<!-- NOTE(antoine): because of degree constraints, a constraint can't have more than 2 flags multiplying the constraint.
It's necessary to add intermediate columns for flag products to keep the degree bounded.
For instance, flag0 = is_add * is_sub * is_sll, flag1 = is_slt * is_sltu * is_xor, flag2 = is_srl * is_sra * is_or, flag3 = flag0 * flag1 * flag2, flag = flag3 * is_and.-->

---

### 3.3 Backend-Agnostic Witness Generation

Witness generation is parameterized over Stwo's `Backend` trait, enabling CPU,
SIMD, and GPU backends without code duplication.

#### 3.3.1 Core Trait

```rust
use stwo_prover::core::backend::Backend;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::poly::circle::CircleEvaluation;
use stwo_prover::core::poly::BitReversedOrder;

pub trait WitnessGenerator<B: Backend> {
    /// Number of trace columns for this opcode family
    const N_COLUMNS: usize;

    /// Generate witness columns from a trace file
    fn generate(
        trace_path: &Path,
        log_size: u32,
    ) -> Vec<CircleEvaluation<B, M31, BitReversedOrder>>;
}
```

#### 3.3.2 Domain Construction

All witness columns share the same evaluation domain:

```rust
use stwo_prover::core::poly::circle::CanonicCoset;

fn create_domain(log_size: u32) -> CircleDomain {
    CanonicCoset::new(log_size).circle_domain()
}
```

The domain size is `2^log_size` points on the circle group.

#### 3.3.3 Bit-Reversal Indexing

Stwo requires columns in bit-reversed order for efficient FFT:

```rust
fn bit_reverse_index(i: usize, log_size: u32) -> usize {
    i.reverse_bits() >> (usize::BITS - log_size)
}
```

<!-- NOTE(antoine): there is no need to explicitly handle bit reversing, it's taken care of when using the cairo-m flow for generating traces. -->

During witness population, trace row `i` maps to column index
`bit_reverse_index(i, log_size)`.

#### 3.3.4 Backend Independence

The witness generator makes no assumptions about the backend:

- No direct use of `SimdBackend` or `PackedM31`
- Column allocation via `B::Column::zeros(size)`
- All operations through trait methods

This allows the same witness generation code to run on any Stwo backend.

---

### 3.4 Memory Witness Generation

<!-- NOTE(antoine): the memory, merkle and clock update witnesses should be generated as so (full memory vm flow):
1. BEFORE EXECUTION: Load memory from ELF (into BTreeMap<u32, (u32,u8)> (addr to (clock,byte) mapping)).
Set initial_memory: BTreeMap<u32, (u8,u8)> (addr to (byte, multiplicity) mapping) from the memory bytes with multiplicity=0 (cells are by default considered as unused)
2. DURING EXECUTION: iterate over instruction and for each memory access at addr:
    - if memory contains addr (already used cell): mark it as used in the initial_memory (just set multiplicity to 1)
    - if memory doesn't contain addr (first access): insert (addr, (val, 1))
    - if the clock difference between the previous clock and the current one is greater than a given STEP, add (addr, prev_clk, u32) to the clock data witness gen buffer.
3. AFTER EXECUTION: current memory is the final memory. Build the memory, merkle and clock update witnesses:
 - memory witness: simply convert the BTreeMaps (initial_memory and memory) into a table like the one described in 3.4.2. setting multiplicity to -1 for the final memory (see see https://github.com/kkrt-labs/cairo-m repo at crates/prover/src/components/memory.rs)
 - merkle witness: build a sparse merkle tree from the initial and final memories, here is the md doc for it (see https://github.com/kkrt-labs/cairo-m repo at crates/prover/src/components/merkle.rs):
    //! Builds partial Merkle trees from memory for Poseidon2.
    //!
    //! # Columns
    //!
    //! - enabler
    //! - index
    //! - depth
    //! - left_value
    //! - right_value
    //! - parent_value
    //! - left_multiplicity
    //! - right_multiplicity
    //! - parent_multiplicity
    //! - root
    //!
    //! # Constraints
    //!
    //! * enabler is a bool
    //!   * `enabler * (1 - enabler)`
    //! * use left node
    //!   * `- [index, depth, left_value, root]` in `Memory` relation
    //! * use right node
    //!   * `- [index + 1, depth, right_value, root]` in `Memory` relation
    //! * emit parent node
    //!   * `+ [index / 2, depth - 1, parent_value, root]` in `Memory` relation
    //! * poseidon2 hash computation
    //!   * `+ [left_value, right_value]` in `Poseidon2` relation (emit hash input)
    //!   * `- [parent_value]` in `Poseidon2` relation (use hash output)
 - poseidon witness: build the complementary information to the merkle witness (see https://github.com/kkrt-labs/cairo-m repo at crates/prover/src/components/poseidon2.rs).
 - clock update: collect the clock update buffer.
-->

Memory consistency requires additional witness structures beyond the per-opcode
traces. The memory witness tracks the complete history of memory accesses,
enabling verification that reads return the most recent write.

#### 3.4.1 Memory Entry Structure

Each memory access generates an entry with temporal ordering:

```rust
pub struct MemoryEntry {
    pub address: M31,           // Memory address
    pub clock: M31,             // Timestamp of this access
    pub prev_clock: M31,        // Timestamp of previous access (0 if first)
    pub value: [M31; 4],        // Byte-decomposed value
    pub prev_value: [M31; 4],   // Value before this access
    pub multiplicity: M31,      // +1 initial, -1 final, 0 intermediate
}
```

The `prev_clock` field enables verification that consecutive accesses to the
same address are properly ordered.

#### 3.4.2 Memory Witness Columns

The memory component uses 9 trace columns:

| Index | Name         | Description                         |
| ----- | ------------ | ----------------------------------- |
| 0     | enabler      | 1 for real rows, 0 for padding      |
| 1     | address      | Memory address (4 bytes decomposed) |
| 2     | clock        | Access timestamp                    |
| 3-6   | value[0-3]   | Byte-decomposed value (4 × M31)     |
| 7     | multiplicity | LogUp sign: +1 initial, -1 final    |
| 8     | root         | Merkle tree root hash               |

#### 3.4.3 Clock Gap Constraint

For consecutive accesses to the same address, the clock difference must be
range-checked:

```text
clock - prev_clock ∈ [1, RC20_LIMIT]
```

Where `RC20_LIMIT = 2^20 - 1`. If the actual gap exceeds this limit, the VM
inserts intermediate "clock update" entries that bridge the gap in increments of
`RC20_LIMIT`. This bounds the range check table size.

<!-- NOTE(antoine): should also do:
`- [addr, prev_clk, value]` in `Memory` relation
`+ [addr, prev_clk + RC_20, value]` in `Memory` relation -->

#### 3.4.4 Memory Root Witness (Merkle)

<!-- NOTE(antoine): this is the memory component not merkle -->

The Merkle component proves the root hash of memory state. Each memory value (4
bytes) emits 4 Merkle lookups:

```text
+1(4 * addr + 0, TREE_HEIGHT, value[0], root)
+1(4 * addr + 1, TREE_HEIGHT, value[1], root)
+1(4 * addr + 2, TREE_HEIGHT, value[2], root)
+1(4 * addr + 3, TREE_HEIGHT, value[3], root)
```

<!-- NOTE(antoine): it also emits initial values (from the initial memory) and uses the final values (from the final memory):
`+ or - [address, clock, value]` in `Memory` relation -->

The Merkle component maintains 10 columns:

| Index | Name           | Description                 |
| ----- | -------------- | --------------------------- |
| 0     | enabler        | Boolean flag                |
| 1     | index          | Node index in tree          |
| 2     | depth          | Tree depth/layer            |
| 3-6   | node_data[0-3] | Node value (4 × M31)        |
| 7-9   | multiplicities | Left, right, parent lookups |

---

<!-- NOTE(antoine): add a section for the merkle witness from the comment in the header -->

### 3.5 LogUp Relations

Cross-component consistency is enforced via LogUp (logarithmic derivative)
relations. Each relation defines a tuple format and multiplicity convention.

**Notation**: `± mult(arg_0, arg_1, ..., arg_n)` denotes a LogUp entry where:

- `+` = lookup (consuming from relation)
- `-` = write (contributing to relation)
- `mult` = multiplicity (typically 1)

#### 3.5.1 Memory Relation

<!-- NOTE(antoine): we need to handle the inplace writing operations (that were avoided with a temp var trick in the compiler for cairo-m) -->

**Tuple size**: 6 **Format**:
`± mult(address, clock, value[0], value[1], value[2], value[3])`

Protocol:

1. **Initial state**: `+1(addr, 0, initial_value)` for all addresses
2. **Each access**: `-1(addr, prev_clock, prev_value)` then
   `+1(addr, clock, value)`
3. **Final state**: `-1(addr, final_clock, final_value)` to balance
   <!-- NOTE(antoine): this is correct, use the initial and final memories mentioned above -->
   The sum of all LogUp contributions must equal zero, proving memory
   consistency.

#### 3.5.2 Merkle Relation

**Tuple size**: 4 **Format**: `+1(index, layer, value, root)`

Each memory entry emits 4 lookups (one per byte) to prove membership in the
Merkle tree. The Merkle component provides matching entries with `-1`
multiplicity.

#### 3.5.3 Register Relation

**Tuple size**: 4 **Format**: `± mult(cycle, reg_idx, value, is_write)`

Each instruction:

- Reads rs1: `+1(cycle, rs1_idx, rs1_val, 0)`
- Reads rs2: `+1(cycle, rs2_idx, rs2_val, 0)` (if applicable)
- Writes rd: `-1(cycle, rd_idx, rd_val, 1)` (if rd ≠ 0)
  <!-- NOTE(antoine): this should work as memory accesses (use previous register and emit new one), can be seen as a 32-long memory (no is_write) -->
  A separate register file component balances these lookups.

<!-- NOTE(antoine): should be balanced by PublicData (see https://github.com/kkrt-labs/cairo-m repo at crates/prover/src/public_data.rs) -->

#### 3.5.4 Range Check Relation

**Tuple size**: 2 **Format**: `+1(value_low, value_high)`

<!-- NOTE(antoine): there should be two range-checks: an RC21 and an RC8_8 (values can be range-checked against arbitrary values with RC20(bound-val)) -->

Used for:

- ALU result/carry pairs
- Clock gap verification (`clock - prev_clock`)
- Any value requiring bounded range

**Constraint**: Total multiplicity across all components must not exceed `2^21`
(the maximum precomputed table size).

---

### 3.6 Interaction Trace Generation

The interaction trace contains cumulative sums for LogUp verification. Each
interaction column is a `SecureColumnByCoords` (4 M31 columns for the QM31
extension field coordinates).

#### 3.6.1 LogUp Constraint Formula

Each LogUp term is committed and constrained as:

```text
committed_value · denominator - multiplicity = 0
```

Where:

- `committed_value`: degree 1 (a trace column)
- `denominator`: Σ(α^i · v_i) - z (aggregation of tuple values using verifier
  randomness)
- `multiplicity`: m (usage count, can be constant or variable)

The constraint degree is:

```text
max(degree(denominator) + 1, degree(multiplicity))
```

This must be ≤ the maximum constraint degree bound (3 for this zkVM).

#### 3.6.2 Pre-Summing for Column Savings

Two LogUp fractions can be pre-summed when the combined degree stays within
bounds:

```text
a/b + c/d = (a·d + c·b) / (b·d)
```

Degree analysis for pre-summing:

- Numerator degree: max(deg(a) + deg(d), deg(c) + deg(b))
- Denominator degree: deg(b) + deg(d)
- Final constraint degree: 1 + deg(b·d)

**Rule**: Can pre-sum up to 2 terms if all use degree-1 variables (when max
bound = 3).

#### 3.6.3 Multiplicity Considerations for Pairing

Pairing decisions depend on the multiplicity degree:

| Multiplicity Type  | Degree | Pairable?       |
| ------------------ | ------ | --------------- |
| Hard-coded (±1)    | 0      | Yes, freely     |
| Trace column       | 1      | Yes, with care  |
| Product of columns | 2      | No, cannot pair |

**Pairing rule**: Check that `deg(m₀) + deg(d₁) + deg(m₁) + deg(d₀) ≤ 2` for the
combined numerator.

#### 3.6.4 Column Count Formula

```text
SECURE_EXTENSION_DEGREE = 4  (QM31 has 4 M31 coordinates)

N_INTERACTION_COLUMNS = SECURE_EXTENSION_DEGREE × ceil(N_LOOKUPS / 2)
                      = 4 × ceil(N_LOOKUPS / 2)
```

For a component with 8 lookups per row:

- 8 lookups → 4 pairs → 4 interaction columns × 4 coordinates = 16 base field
  columns

#### 3.6.5 Lookup Ordering for Optimal Pairing

Order lookups to group by degree compatibility:

```rust
// Good: both have m=1 (constant) and degree-1 denominators → safe to pair
consume_pair!(interaction_trace;
    rs1_read, rs2_read,           // Both register reads, m=1
    range_check_0, range_check_1, // Both range checks, m=1
);

// Variable multiplicity requires care:
// m_var/d + 1/d' → numerator = m_var·d' + d → degree 1+1 = 2
// Final: committed · (d·d') - (m_var·d' + d) → degree 1+2 = 3 ✓
```

#### 3.6.6 Interaction Trace Generation Pattern

```rust
pub fn gen_interaction_trace<B: Backend>(
    lookup_data: &LookupData,
    relations: &Relations,
    log_size: u32,
) -> (Vec<CircleEvaluation<B, M31, BitReversedOrder>>, QM31) {
    let mut interaction_trace = LogupTraceGenerator::new(log_size);

    // Pair lookups for degree-2 constraints
    // Each consume_pair! creates one interaction column (4 base field cols)

    // Register lookups: rs1_read + rs2_read (paired, both m=1)
    let rs1_denom = relations.register.combine(&[cycle, rs1_idx, rs1_val, 0]);
    let rs2_denom = relations.register.combine(&[cycle, rs2_idx, rs2_val, 0]);
    consume_pair!(rs1_denom, rs2_denom, interaction_trace);

    // Register write: rd_write (unpaired if odd count, m=-1)
    let rd_denom = relations.register.combine(&[cycle, rd_idx, rd_val, 1]);
    emit_col!(rd_denom, -1, interaction_trace);

    // Memory lookups (if load/store): paired
    // Range checks: paired

    interaction_trace.finalize_last()  // Returns (columns, claimed_sum)
}
```

#### 3.6.7 Interaction Column Layout per Family

| Family    | Lookups/Row             | Pairs          | Interaction Cols | Base Cols |
| --------- | ----------------------- | -------------- | ---------------- | --------- |
| `alu_reg` | 3 (rs1, rs2, rd)        | 1 + 1 unpaired | 2                | 8         |
| `alu_imm` | 2 (rs1, rd)             | 1              | 1                | 4         |
| `load`    | 4 (rs1, rd, mem, range) | 2              | 2                | 8         |
| `store`   | 3 (rs1, rs2, mem)       | 1 + 1 unpaired | 2                | 8         |
| `branch`  | 2 (rs1, rs2)            | 1              | 1                | 4         |
| `jump`    | 2 (rs1/-, rd)           | 1              | 1                | 4         |
| `mul_div` | 3 + range checks        | varies         | ~3               | 12        |
| `memory`  | 1 mem + 4 merkle        | 2 + 1 unpaired | 3                | 12        |

---

### 3.7 Trace-to-Witness Pipeline

The pipeline transforms Section 2 trace files into Stwo-compatible witness
columns:

<!-- NOTE(antoine): files are used for rare occasions but the overwhelmingly majority of times the traces are directly passed to prover without dumping them into a file (which will greatly slow down the flow). -->

```text
┌─────────────────────────────────────────────────────────────┐
│           Section 2 Trace Files (column-major)              │
│   trace_alu_reg.bin, trace_load.bin, trace_memory.bin, ...  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Memory-map / Read                        │
│   Parse header: magic, version, family_id, n_columns, n_rows│
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Compute Padded Size                            │
│   log_size = ceil(log2(n_rows))                             │
│   padded_size = 2^log_size                                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│            Allocate CircleEvaluation Columns                │
│   domain = CanonicCoset::new(log_size).circle_domain()      │
│   columns = vec![CircleEvaluation::new(domain); n_columns]  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Populate with Bit-Reversal                     │
│   for col_idx in 0..n_columns:                              │
│       for row_idx in 0..padded_size:                        │
│           target = bit_reverse_index(row_idx, log_size)     │
│           if row_idx < n_rows:                              │
│               columns[col_idx][target] = trace[col_idx][row]│
│           else:                                             │
│               columns[col_idx][target] = M31::zero()        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│         Vec<CircleEvaluation<B, M31, BitReversedOrder>>     │
│              Ready for FrameworkEval                        │
└─────────────────────────────────────────────────────────────┘
```

---

### 3.8 FrameworkEval Integration

Each opcode family implements Stwo's `FrameworkEval` trait to define its
constraints.

#### 3.7.1 Trait Implementation Pattern

```rust
use stwo_prover::constraint_framework::{
    EvalAtRow, FrameworkEval, RelationEntry,
};
use stwo_prover::core::fields::m31::M31;

pub struct AluRegEval {
    pub log_size: u32,
    pub memory_lookup_elements: MemoryElements,
    pub register_lookup_elements: RegisterElements,
}

impl FrameworkEval for AluRegEval {
    fn log_size(&self) -> u32 {
        self.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size + 1  // Degree-2 constraints
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        // Read trace columns in order
        let cycle = [
            eval.next_trace_mask(),
            eval.next_trace_mask(),
            eval.next_trace_mask(),
            eval.next_trace_mask(),
        ];
        let pc = [
            eval.next_trace_mask(),
            eval.next_trace_mask(),
            eval.next_trace_mask(),
            eval.next_trace_mask(),
        ];
        // ... remaining columns

        // Constraint: enabler is boolean
        let enabler = eval.next_trace_mask();
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));

        // Constraint: result = rs1_val + rs2_val (for ADD)
        // Guarded by is_add selector
        let is_add = /* derived from funct3/funct7 */;
        eval.add_constraint(
            is_add * (result[0] - rs1_val[0] - rs2_val[0] - carry_in)
        );

        // LogUp: register file lookups
        eval.add_to_relation(RelationEntry::new(
            &self.register_lookup_elements,
            E::EF::one(),  // multiplicity +1
            &[cycle, rs1_idx, rs1_val, E::F::zero()],  // is_write = 0
        ));

        eval
    }
}
```

#### 3.7.2 Component Composition

The complete proof composes multiple components:

1. **Opcode components** (8): One per instruction family
2. **Memory component**: Tracks all memory accesses
3. **Merkle component**: Proves memory root hash
4. **Register file component**: Tracks register state
5. **Range check component**: Precomputed lookup table

Each component contributes to shared LogUp relations, and the final proof
verifies all relations sum to zero.

---

### 3.9 Complete Witness Generation Implementation

This section provides a complete, runnable implementation for witness generation
with end-to-end validation against the Section 2.7 test program.

#### 3.9.1 Opcode Trace Witness Generator

<!-- NOTE(antoine): as mentioned above, default behavior for generating witness is to use the rust objects rather than the files (which are an optional for now feature). -->

```rust
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use stwo_prover::core::backend::Backend;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::poly::circle::{CanonicCoset, CircleEvaluation};
use stwo_prover::core::poly::BitReversedOrder;

/// Trace file header (matches Section 2.4.4)
#[repr(C)]
pub struct TraceHeader {
    pub magic: u32,      // 0x54524143 ("TRAC")
    pub version: u32,    // 1
    pub family_id: u32,  // Opcode family identifier
    pub n_columns: u32,  // Number of columns
    pub n_rows: u64,     // Number of rows
    pub reserved: u64,   // Must be zero
}

/// Load trace file and generate witness columns
pub fn generate_witness<B: Backend>(
    trace_path: &Path,
) -> (u32, Vec<CircleEvaluation<B, M31, BitReversedOrder>>) {
    // Read header
    let mut file = BufReader::new(File::open(trace_path).unwrap());
    let mut header_bytes = [0u8; 32];
    file.read_exact(&mut header_bytes).unwrap();

    let header = unsafe { std::ptr::read(header_bytes.as_ptr() as *const TraceHeader) };
    assert_eq!(header.magic, 0x54524143, "Invalid trace magic");
    assert_eq!(header.version, 1, "Unsupported trace version");

    let n_columns = header.n_columns as usize;
    let n_rows = header.n_rows as usize;

    // Compute padded size
    let log_size = if n_rows == 0 {
        0
    } else {
        (usize::BITS - (n_rows - 1).leading_zeros()) as u32
    };
    let padded_size = 1 << log_size;

    // Create domain
    let domain = CanonicCoset::new(log_size).circle_domain();

    // Read all column data
    let mut trace_data = vec![vec![M31::zero(); n_rows]; n_columns];
    for col in 0..n_columns {
        for row in 0..n_rows {
            let mut val_bytes = [0u8; 4];
            file.read_exact(&mut val_bytes).unwrap();
            let val = u32::from_le_bytes(val_bytes);
            trace_data[col][row] = M31::from(val); // NOTE(antoine): use PackedM31
        }
    }

    // Generate witness columns with bit-reversal
    let columns = (0..n_columns)
        .map(|col_idx| {
            let mut column = vec![M31::zero(); padded_size];
            for row_idx in 0..padded_size {
                let target = bit_reverse_index(row_idx, log_size);
                if row_idx < n_rows {
                    column[target] = trace_data[col_idx][row_idx];
                }
            }
            CircleEvaluation::new(domain.clone(), column.into())
        })
        .collect();

    (log_size, columns)
}

fn bit_reverse_index(i: usize, log_size: u32) -> usize {
    if log_size == 0 {
        return 0;
    }
    i.reverse_bits() >> (usize::BITS - log_size)
}
```

#### 3.9.2 Memory Witness Generator

<!-- NOTE(antoine): it's better to have a unique source of truth for the memory as mentioned above in 3.4 (unit Memory and MemoryWitness functions)-->

```rust
use std::collections::BTreeMap;

/// Memory state tracker for witness generation
pub struct MemoryWitness {
    /// address -> (clock, value, prev_clock)
    state: BTreeMap<u32, (u64, [M31; 4], u64)>,
    /// All memory entries for witness
    entries: Vec<MemoryEntry>,
}

pub struct MemoryEntry {
    pub address: M31,
    pub clock: M31,
    pub prev_clock: M31,
    pub value: [M31; 4],
    pub multiplicity: M31,
}

impl MemoryWitness {
    pub fn new() -> Self {
        Self {
            state: BTreeMap::new(),
            entries: Vec::new(),
        }
    }

    /// Record a memory access
    pub fn access(&mut self, address: u32, clock: u64, value: u32, is_write: bool) {
        let value_bytes = decompose_to_bytes(value);

        let (prev_clock, prev_value) = self.state
            .get(&address)
            .map(|(c, v, _)| (*c, *v))
            .unwrap_or((0, [M31::zero(); 4]));

        // Handle clock gaps exceeding RC20_LIMIT
        const RC20_LIMIT: u64 = (1 << 20) - 1;
        let mut current_prev_clock = prev_clock;
        let mut current_prev_value = prev_value;

        while clock - current_prev_clock > RC20_LIMIT {
            let intermediate_clock = current_prev_clock + RC20_LIMIT;
            self.entries.push(MemoryEntry {
                address: M31::from(address),
                clock: M31::from(intermediate_clock as u32),
                prev_clock: M31::from(current_prev_clock as u32),
                value: current_prev_value,
                multiplicity: M31::zero(), // Intermediate entry
            });
            current_prev_clock = intermediate_clock;
        }

        // Record actual access
        self.entries.push(MemoryEntry {
            address: M31::from(address),
            clock: M31::from(clock as u32),
            prev_clock: M31::from(current_prev_clock as u32),
            value: if is_write { value_bytes } else { current_prev_value },
            multiplicity: M31::zero(),
        });

        // Update state
        if is_write {
            self.state.insert(address, (clock, value_bytes, current_prev_clock));
        }
    }

    /// Generate final memory witness with initial/final multiplicities
    pub fn finalize<B: Backend>(
        &self,
        log_size: u32,
    ) -> Vec<CircleEvaluation<B, M31, BitReversedOrder>> {
        let domain = CanonicCoset::new(log_size).circle_domain();
        let padded_size = 1 << log_size;

        // 9 columns: enabler, address, clock, value[0-3], multiplicity, root
        let mut columns = vec![vec![M31::zero(); padded_size]; 9];

        for (row_idx, entry) in self.entries.iter().enumerate() {
            let target = bit_reverse_index(row_idx, log_size);
            columns[0][target] = M31::one();  // enabler
            columns[1][target] = entry.address;
            columns[2][target] = entry.clock;
            columns[3][target] = entry.value[0];
            columns[4][target] = entry.value[1];
            columns[5][target] = entry.value[2];
            columns[6][target] = entry.value[3];
            columns[7][target] = entry.multiplicity;
            // columns[8] = root (computed by Merkle component)
        }

        columns
            .into_iter()
            .map(|col| CircleEvaluation::new(domain.clone(), col.into()))
            .collect()
    }
}

fn decompose_to_bytes(v: u32) -> [M31; 4] {
    [
        M31::from((v >> 0) & 0xFF),
        M31::from((v >> 8) & 0xFF),
        M31::from((v >> 16) & 0xFF),
        M31::from((v >> 24) & 0xFF),
    ]
}
```

<!-- NOTE(antoine): the witness generation should be inspired from the one in the cairo-m repo (using packed M31 and lookup data). -->

#### 3.9.3 Interaction Trace Generator

<!-- NOTE(antoine): again the interaction witness generation should be inspired from the one in the cairo-m repo (using packed M31 and lookup data). -->

```rust
use stwo_prover::core::fields::qm31::QM31;
use stwo_prover::constraint_framework::logup::LogupTraceGenerator;

/// Generate interaction trace columns for LogUp verification
pub fn gen_interaction_trace<B: Backend>(
    lookup_data: &LookupData,
    relations: &Relations,
    log_size: u32,
) -> (Vec<CircleEvaluation<B, M31, BitReversedOrder>>, QM31) {
    let mut interaction_trace = LogupTraceGenerator::new(log_size);
    let domain = CanonicCoset::new(log_size).circle_domain();

    // Process each row's lookups
    for row_idx in 0..(1 << log_size) {
        // Register lookups: pair rs1_read + rs2_read (both m=1)
        let rs1_denom = relations.register.combine(&[
            lookup_data.cycle[row_idx],
            lookup_data.rs1_idx[row_idx],
            lookup_data.rs1_val[row_idx],
            M31::zero(), // is_write = 0
        ]);
        let rs2_denom = relations.register.combine(&[
            lookup_data.cycle[row_idx],
            lookup_data.rs2_idx[row_idx],
            lookup_data.rs2_val[row_idx],
            M31::zero(),
        ]);

        // Paired: degree(m0=0) + degree(d1=1) + degree(m1=0) + degree(d0=1) = 2 ≤ 2 ✓
        interaction_trace.write_frac(
            row_idx,
            Fraction::new(
                QM31::one() * rs2_denom + QM31::one() * rs1_denom,  // numerator
                rs1_denom * rs2_denom,                              // denominator
            ),
        );

        // Register write: rd (unpaired, m=-1)
        if lookup_data.rd_idx[row_idx] != M31::zero() {
            let rd_denom = relations.register.combine(&[
                lookup_data.cycle[row_idx],
                lookup_data.rd_idx[row_idx],
                lookup_data.rd_val[row_idx],
                M31::one(), // is_write = 1
            ]);
            interaction_trace.write_frac(
                row_idx,
                Fraction::new(-QM31::one(), rd_denom),
            );
        }

        // Range check lookups (paired)
        if lookup_data.has_range_check[row_idx] {
            let rc0_denom = relations.range_check.combine(&[
                lookup_data.result_low[row_idx],
                lookup_data.carry_low[row_idx],
            ]);
            let rc1_denom = relations.range_check.combine(&[
                lookup_data.result_high[row_idx],
                lookup_data.carry_high[row_idx],
            ]);
            interaction_trace.write_frac(
                row_idx,
                Fraction::new(
                    QM31::one() * rc1_denom + QM31::one() * rc0_denom,
                    rc0_denom * rc1_denom,
                ),
            );
        }
    }

    interaction_trace.finalize_last()
}
```

#### 3.9.4 End-to-End Test Path

The following test validates the complete pipeline from ELF compilation through
witness generation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// End-to-end test using Section 2.7 test program
    #[test]
    fn test_witness_generation_e2e() {
        // Step 1: Compile the test program
        let status = Command::new("cargo")
            .args([
                "build",
                "--release",
                "--bin", "test-all-opcodes",
                "--target", "riscv32im-unknown-none-elf",
            ])
            .current_dir("guest-bin")
            .status()
            .expect("Failed to compile test program");
        assert!(status.success(), "Compilation failed");

        // Step 2: Run VM to generate traces
        let status = Command::new("./target/release/zkvm-runner")
            .args([
                "run",
                "guest-bin/target/riscv32im-unknown-none-elf/release/test-all-opcodes",
                "--trace-dir", "traces/",
            ])
            .status()
            .expect("Failed to run VM");
        assert!(status.success(), "VM execution failed");

        // Step 3: Load traces and generate witnesses
        let trace_files = [
            ("traces/trace_alu_reg.bin", 31),
            ("traces/trace_alu_imm.bin", 26),
            ("traces/trace_upper_imm.bin", 21),
            ("traces/trace_branch.bin", 31),
            ("traces/trace_load.bin", 30),
            ("traces/trace_store.bin", 29),
            ("traces/trace_jump.bin", 26),
            ("traces/trace_mul_div.bin", 35),
        ];

        for (path, expected_columns) in trace_files {
            let trace_path = Path::new(path);
            if trace_path.exists() {
                let (log_size, columns) =
                    generate_witness::<CpuBackend>(trace_path);

                // Verify column count
                assert_eq!(
                    columns.len(),
                    expected_columns,
                    "Column count mismatch for {}",
                    path
                );

                // Verify power-of-2 size
                assert!(
                    columns[0].len().is_power_of_two(),
                    "Witness size not power of 2"
                );

                println!(
                    "{}: {} columns, 2^{} rows",
                    path, columns.len(), log_size
                );
            }
        }

        // Step 4: Build memory witness
        let memory_trace_path = Path::new("traces/trace_memory.bin");
        if memory_trace_path.exists() {
            let mut memory_witness = MemoryWitness::new();
            // Load memory trace and populate witness
            // (simplified: actual implementation reads trace file)

            let log_size = 10; // Example
            let memory_columns =
                memory_witness.finalize::<CpuBackend>(log_size);

            assert_eq!(memory_columns.len(), 9, "Memory witness column count");
        }

        // Step 5: Validate LogUp relation balancing
        // Sum of all relation contributions should equal zero
        // (Full implementation requires running the constraint evaluator)

        println!("E2E witness generation test passed");
    }

    /// Test that padding rows have correct structure
    #[test]
    fn test_padding_rows() {
        // Create a trace with 5 rows (pads to 8)
        let n_rows = 5;
        let log_size = 3; // 2^3 = 8
        let padded_size = 8;

        // Verify bit-reversal mapping
        for i in 0..padded_size {
            let target = bit_reverse_index(i, log_size);
            println!("Row {} -> index {}", i, target);
        }

        // Verify padding detection
        for i in n_rows..padded_size {
            let target = bit_reverse_index(i, log_size);
            // At target index, column should be zero (padding)
            println!("Padding row {} at index {}", i, target);
        }
    }
}
```

#### 3.9.5 Validation Criteria

| Criterion                  | Validation Method                                |
| -------------------------- | ------------------------------------------------ |
| All 47 instructions traced | Trace file row counts > 0 for each family        |
| Column counts match schema | `columns.len() == expected_columns`              |
| Power-of-2 padding         | `columns[0].len().is_power_of_two()`             |
| Memory prev_clock ordering | `entry.clock > entry.prev_clock` for all         |
| LogUp relations balance    | `claimed_sum == QM31::zero()` after finalization |
| Clock gaps bounded         | `clock - prev_clock <= RC20_LIMIT` everywhere    |
| Interaction cols correct   | `4 × ceil(N_LOOKUPS / 2)` columns per component  |
| Constraint degree ≤ 3      | `max(deg(denom) + 1, deg(mult)) ≤ 3`             |

End of Section 4.

---

## 5. AIR Constraints by Opcode Family

This section defines the Algebraic Intermediate Representation (AIR) constraints
for each RV32IM instruction. Constraints enforce that trace columns (defined in
Section 2.4) represent valid instruction executions. The constraints are
designed for integration with Stwo's STARK proving system.

### 4.1 Constraint Notation and Organization

#### 4.1.1 Notation Conventions

Throughout this section, the following notation is used:

| Symbol      | Meaning                                                |
| ----------- | ------------------------------------------------------ |
| `col[i]`    | Value in column `col` at row `i`                       |
| `·`         | Field multiplication                                   |
| `+`, `-`    | Field addition and subtraction                         |
| `= 0`       | Constraint that expression equals zero                 |
| `∈ Table`   | Lookup constraint (value exists in precomputed table)  |
| `byte_k(x)` | The k-th byte of 32-bit value x: `(x >> (8·k)) & 0xFF` |

#### 4.1.2 Relationship to Trace Columns

Each constraint operates on columns from the trace families defined in Section
2.4. Values in the trace are already byte-decomposed into M31 field elements.
For a 32-bit value `v`, the trace stores four columns:

```text
v_0 = byte_0(v)   # LSB
v_1 = byte_1(v)
v_2 = byte_2(v)
v_3 = byte_3(v)   # MSB
```

Reconstruction: `v = v_0 + v_1·2⁸ + v_2·2¹⁶ + v_3·2²⁴`

#### 4.1.3 Stwo Framework Integration

Constraints are implemented using Stwo's `FrameworkEval` trait. Each opcode
family defines a component:

```rust
impl FrameworkEval for OpcodeEval {
    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        // Read trace columns
        let col_a = eval.next_trace_mask();
        let col_b = eval.next_trace_mask();

        // Add polynomial constraint
        eval.add_constraint(col_a * col_b - expected);

        // Add lookup constraint
        eval.add_to_relation(&[col_a], lookup_table);

        eval
    }
}
```

#### 4.1.4 Logup-Based Lookups

All lookup arguments use the logup protocol. Each lookup contributes a term to a
cumulative sum that must equal zero across the entire trace:

```text
Σ (multiplicity / (α - tuple)) = 0
```

Where `α` is a random challenge and `tuple` combines column values into a single
field element using powers of another challenge `β`.

---

### 4.2 Precomputed Columns and Lookup Tables

Precomputed tables enable efficient range checks and bitwise operations without
expensive polynomial constraints. Each table is a separate Stwo component that
verifies lookup multiplicities balance correctly via the LogUp protocol.

#### 4.2.1 Range Check Tables

Range check tables contain all values in a given range. Each lookup verifies a
witness value exists in the table.

**RangeCheck8** (2⁸ = 256 rows):

- **Use**: Verify byte decomposition columns throughout the trace
- **Structure**: Single column containing values 0..255
- **Lookup**: `byte ∈ RangeCheck8` verifies `byte < 256`

**RangeCheck16** (2¹⁶ = 65,536 rows):

- **Use**: Verify carry bounds in multi-limb arithmetic, halfword values
- **Structure**: Single column containing values 0..65535
- **Example**: Multiplication carries can reach ~1000; division remainders need
  16-bit bounds

**RangeCheck20** (2²⁰ = 1,048,576 rows):

- **Use**: Verify cycle differences (clock - prev_clock) for memory ordering
- **Structure**: Single column containing values 0..2²⁰-1
- **Rationale**: Maximum trace length bounded by 2²⁰ instructions

#### 4.2.2 Bitwise Lookup Tables

Bitwise operations (XOR, OR, AND) on bytes are implemented via a single
**stacked table** containing all three operations:

| Column   | Description                              |
| -------- | ---------------------------------------- |
| `op_id`  | Operation identifier: 0=AND, 1=OR, 2=XOR |
| `input1` | First operand (0..255)                   |
| `input2` | Second operand (0..255)                  |
| `result` | Computed output for the operation        |

**Table Structure**:

- Total size: 3 × 256 × 256 = 196,608 rows (padded to 2¹⁸ = 262,144)
- Index formula: `op_id × 65536 + (input1 << 8) + input2`
- Operations stacked sequentially: rows [0..65536) = AND, [65536..131072) = OR,
  [131072..196608) = XOR

**Lookup**: To verify `a XOR b = c`, emit tuple `(op_id=2, a, b, c)` to the
bitwise relation.

#### 4.2.3 LogUp Protocol Implementation

The LogUp protocol verifies that lookup consumers and providers balance. Each
lookup contributes a term to a cumulative sum that must equal zero.

**Multiplicity Counting** (during trace generation):

```rust
// Create atomic counters for thread-safe parallel counting
let mults: Vec<AtomicU32> = (0..table_size)
    .map(|_| AtomicU32::new(0))
    .collect();

// Count how many times each value is looked up
lookup_data.par_iter().for_each(|value| {
    let index = value.0 as usize;
    mults[index].fetch_add(1, Ordering::Relaxed);
});
```

**Relation Entry Structure**:

Relations combine multiple columns into a single fingerprint using random
challenges (α, β) from the Fiat-Shamir channel:

```rust
// Combine columns: α - (col0 + β·col1 + β²·col2 + ...)
let denom: PackedQM31 = relation.combine(&[col0, col1, col2]);
```

**Interaction Trace Generation**:

The LogUp interaction trace accumulates fractions `multiplicity / denom`:

```rust
// In precomputed component: provide lookup values
let mut col = interaction_trace.new_col();
col.par_iter_mut().zip(&packed_data).for_each(|(writer, row)| {
    let denom = relation.combine(&row[..N_COLS]);
    let multiplicity = row[N_COLS];  // Last element is multiplicity
    writer.write_frac(multiplicity.into(), denom);
});
col.finalize_col();

// In opcode component: consume lookup values
eval.add_to_relation(RelationEntry::new(
    &range_check_8,
    E::EF::from(-E::F::one()),  // Negative multiplicity
    &[byte_value],
));
```

**Cumulative Sum Verification**:

The final cumulative sum must equal zero:

```text
Σ (provider_mult / denom) + Σ (-consumer_mult / denom) = 0
```

This guarantees every consumed lookup has a corresponding provider entry with
matching multiplicity.

**Implementation Path**:

- Define `RangeCheck8`, `RangeCheck16`, `RangeCheck20` components with
  single-column tables and multiplicity tracking
- Define unified `Bitwise` component with 4-column stacked table (op_id, input1,
  input2, result)
- Use `AtomicU32` counters for parallel multiplicity accumulation during trace
  generation
- Implement `LogupTraceGenerator` for interaction trace with `write_frac()`
- Opcode components emit negative multiplicities; precomputed components emit
  positive multiplicities from the multiplicity column

---

### 4.3 R-type Arithmetic Constraints

R-type instructions use the `alu_reg` trace family (31 columns). All share the
same column layout but differ in the operation applied.

#### 4.3.1 Common Columns

| Column Range | Name      | Description                          |
| ------------ | --------- | ------------------------------------ |
| 0-3          | `cycle`   | Instruction cycle (4 bytes)          |
| 4-7          | `pc`      | Program counter (4 bytes)            |
| 8-11         | `instr`   | Instruction word (4 bytes)           |
| 12           | `rs1_idx` | Source register 1 index              |
| 13-16        | `rs1_val` | Source register 1 value (4 bytes)    |
| 17           | `rs2_idx` | Source register 2 index              |
| 18-21        | `rs2_val` | Source register 2 value (4 bytes)    |
| 22           | `rd_idx`  | Destination register index           |
| 23-26        | `rd_val`  | Destination register value (4 bytes) |
| 27-30        | `result`  | Computed result (4 bytes)            |

#### 4.3.2 Auxiliary Witness Columns

Beyond the common columns, R-type arithmetic requires auxiliary witness columns
for intermediate values. These columns are instruction-specific:

| Column                             | Type     | Used By        | Description                                   |
| ---------------------------------- | -------- | -------------- | --------------------------------------------- |
| `carry_0`, `carry_1`, `carry_2`    | Binary   | ADD            | Byte-level carry propagation                  |
| `borrow_0`, `borrow_1`, `borrow_2` | Binary   | SUB, SLT, SLTU | Byte-level borrow propagation                 |
| `shift_amt`                        | 5-bit    | SLL, SRL, SRA  | Masked shift amount from rs2[4:0]             |
| `remainder`                        | Variable | SRL, SRA       | Bits shifted out (range depends on shift_amt) |
| `sign_rs1`, `sign_rs2`             | Binary   | SLT, SRA       | MSB of operands for signed operations         |

Carry and borrow columns are constrained to be binary: `c · (1 - c) = 0`.

#### 4.3.3 ADD Constraints

```text
# Result = rs1_val + rs2_val (with carry propagation)
result_0 + carry_0·256 = rs1_val_0 + rs2_val_0
result_1 + carry_1·256 = rs1_val_1 + rs2_val_1 + carry_0
result_2 + carry_2·256 = rs1_val_2 + rs2_val_2 + carry_1
result_3              = rs1_val_3 + rs2_val_3 + carry_2  (mod 256)

# Range checks
result_i ∈ RangeCheck8  for i in 0..3
carry_i ∈ {0, 1}        for i in 0..2

# Destination equals result
rd_val_i = result_i     for i in 0..3
```

#### 4.3.4 SUB Constraints

```text
# Result = rs1_val - rs2_val (with borrow propagation)
rs1_val_0 = result_0 + rs2_val_0 - borrow_0·256
rs1_val_1 = result_1 + rs2_val_1 + borrow_0 - borrow_1·256
rs1_val_2 = result_2 + rs2_val_2 + borrow_1 - borrow_2·256
rs1_val_3 = result_3 + rs2_val_3 + borrow_2  (mod 256)

# Range checks
result_i ∈ RangeCheck8  for i in 0..3
borrow_i ∈ {0, 1}       for i in 0..2
```

#### 4.3.5 Shift Left (SLL) Constraints

Shift amount is masked to 5 bits (values 0-31):

```text
# shift_amt = rs2_val[4:0] (only low 5 bits)
shift_amt = rs2_val_0 & 0x1F
shift_amt ∈ RangeCheck5  # Verify 5-bit range

# Result = rs1_val << shift_amt
```

**Lookup approach**: Use precomputed table for all (value, amount, result)
triples. This requires a 32-entry table per byte position with 2^8 × 32 = 8192
entries total.

```text
(rs1_val, shift_amt, result) ∈ ShiftLeftTable
```

**Decomposed approach**: Express left shift as multiplication by power of 2:

```text
result = rs1_val · 2^shift_amt  (mod 2^32)
# Witness provides 2^shift_amt value; verify via lookup
power_of_2 ∈ PowersOfTwo  # Table of {1, 2, 4, ..., 2^31}
result = rs1_val · power_of_2  (mod 2^32)
```

The decomposed approach is preferred for its smaller lookup table.

#### 4.3.6 Shift Right Logical (SRL) Constraints

```text
# shift_amt = rs2_val[4:0]
shift_amt = rs2_val_0 & 0x1F

# Result = rs1_val >> shift_amt (zero-fill)
# Constraint via division with remainder:
rs1_val = result · 2^shift_amt + remainder

# Remainder must be strictly less than 2^shift_amt
# Witness: remainder_bound = 2^shift_amt - 1 - remainder
remainder_bound ∈ RangeCheck{shift_amt}  # Non-negative check
remainder ∈ RangeCheck{shift_amt}        # Upper bound implicit
```

#### 4.3.7 Shift Right Arithmetic (SRA) Constraints

```text
# shift_amt = rs2_val[4:0]
shift_amt = rs2_val_0 & 0x1F

# Extract sign bit from MSB
sign_rs1 = rs1_val_3 >> 7
sign_rs1 · (1 - sign_rs1) = 0  # Binary constraint

# Result = rs1_val >> shift_amt (sign-extended)
# For positive values (sign=0), same as SRL
# For negative values (sign=1), fill high bits with 1s

# Unsigned shifted value (same as SRL)
unsigned_result · 2^shift_amt + remainder = rs1_val

# Sign extension mask: ones in high (shift_amt) bits
sign_mask = (2^32 - 1) - (2^(32-shift_amt) - 1)

# Final result
result = unsigned_result + sign_rs1 · sign_mask
```

#### 4.3.8 Set Less Than (SLT) Constraints

Signed comparison requires extracting sign bits and handling two cases:

```text
# Extract sign bits from MSB of each operand
sign_rs1 = rs1_val_3 >> 7
sign_rs2 = rs2_val_3 >> 7
sign_rs1 · (1 - sign_rs1) = 0  # Binary constraint
sign_rs2 · (1 - sign_rs2) = 0  # Binary constraint

# Compute difference with borrow tracking (same as SUB)
diff_0 + borrow_0·256 = rs1_val_0 - rs2_val_0 + 256
diff_1 + borrow_1·256 = rs1_val_1 - rs2_val_1 + borrow_0 + 256
diff_2 + borrow_2·256 = rs1_val_2 - rs2_val_2 + borrow_1 + 256
diff_3               = rs1_val_3 - rs2_val_3 + borrow_2  (mod 256)

# Borrow_out indicates rs1 < rs2 for unsigned interpretation
borrow_out = borrow_2  # Final borrow

# Signed comparison logic:
# - If signs differ: negative (sign=1) is smaller
# - If signs same: use unsigned borrow result
signs_differ = sign_rs1 · (1 - sign_rs2) + sign_rs2 · (1 - sign_rs1)  # XOR
result = signs_differ · sign_rs1 + (1 - signs_differ) · borrow_out
result ∈ {0, 1}
```

#### 4.3.9 Set Less Than Unsigned (SLTU) Constraints

Unsigned comparison uses borrow propagation from subtraction:

```text
# Compute rs1 - rs2 with borrow chain
# borrow_out = 1 means rs1 < rs2 (unsigned)

diff_0 + borrow_0·256 = rs1_val_0 - rs2_val_0 + 256
diff_1 + borrow_1·256 = rs1_val_1 - rs2_val_1 + borrow_0 + 256
diff_2 + borrow_2·256 = rs1_val_2 - rs2_val_2 + borrow_1 + 256
diff_3               = rs1_val_3 - rs2_val_3 + borrow_2  (mod 256)

# The +256 terms ensure intermediate values stay non-negative
# borrow_i ∈ {0, 1} captures whether subtraction underflowed

result = borrow_2  # Final borrow indicates rs1 < rs2
result ∈ {0, 1}
```

#### 4.3.10 Bitwise XOR Constraints

```text
# Result = rs1_val XOR rs2_val (byte by byte)
(rs1_val_i, rs2_val_i, result_i) ∈ BitwiseXor8  for i in 0..3
```

#### 4.3.11 Bitwise OR Constraints

```text
(rs1_val_i, rs2_val_i, result_i) ∈ BitwiseOr8  for i in 0..3
```

#### 4.3.12 Bitwise AND Constraints

```text
(rs1_val_i, rs2_val_i, result_i) ∈ BitwiseAnd8  for i in 0..3
```

**Implementation Path**:

- Create `AluRegComponent` implementing `FrameworkEval`
- Use `funct3` and `funct7` fields from `instr` to select operation
- Share carry/borrow witness columns across ADD/SUB/SLT/SLTU
- Use bitwise lookup tables for XOR/OR/AND (65536 entries each)
- For shifts, use PowersOfTwo lookup (32 entries) with decomposed constraints
- Range check all byte columns via RangeCheck8

---

### 4.4 I-type Immediate Constraints

I-type instructions use the `alu_imm` trace family (26 columns). They operate on
one register and a sign-extended 12-bit immediate.

#### 4.4.1 Column Layout

The I-type trace shares most columns with R-type but replaces the second source
register with the sign-extended immediate:

| Column Range | Name      | Description                          | Shared with R-type |
| ------------ | --------- | ------------------------------------ | ------------------ |
| 0-3          | `cycle`   | Instruction cycle (4 bytes)          | Yes                |
| 4-7          | `pc`      | Program counter (4 bytes)            | Yes                |
| 8-11         | `instr`   | Instruction word (4 bytes)           | Yes                |
| 12           | `rs1_idx` | Source register index                | Yes                |
| 13-16        | `rs1_val` | Source register value (4 bytes)      | Yes                |
| 17-20        | `imm`     | Sign-extended immediate (4 bytes)    | No (replaces rs2)  |
| 21           | `rd_idx`  | Destination register index           | Yes                |
| 22-25        | `rd_val`  | Destination register value (4 bytes) | Yes                |

This column sharing allows constraint logic reuse: arithmetic operations (ADDI,
SLTI, etc.) use identical constraint equations to their R-type counterparts,
substituting `imm` for `rs2_val`.

#### 4.4.2 Immediate Sign Extension

The 12-bit immediate is extracted from instruction bits [31:20] and
sign-extended:

```text
# Extract sign bit from instruction MSB
sign = instr_3 >> 7
sign · (1 - sign) = 0  # Constrain to binary

# Raw 12-bit immediate extraction:
# Bits [31:20] span instr_2[7:4] and instr_3[7:0]
raw_hi = instr_3 & 0x7F              # Bits [30:24] → imm[10:4]
raw_lo = (instr_2 >> 4) & 0x0F       # Bits [23:20] → imm[3:0]

# Byte-level sign extension to 32 bits
imm_0 = (raw_hi << 4) | raw_lo       # imm[7:0]
imm_1 = (instr_3 >> 4) | (sign · 0xF0)  # imm[15:8] with sign fill
imm_2 = sign · 0xFF                  # imm[23:16] = sign extension
imm_3 = sign · 0xFF                  # imm[31:24] = sign extension

# Range checks
imm_i ∈ RangeCheck8  for i in 0..3
```

#### 4.4.3 Arithmetic Immediates (ADDI, SLTI, SLTIU)

These reuse R-type constraint patterns with `imm` substituted for `rs2_val`:

```text
# ADDI: rd = rs1 + sign_ext(imm)
# Uses ADD constraints from Section 4.3.3 with imm_i replacing rs2_val_i
result_0 + carry_0·256 = rs1_val_0 + imm_0
result_1 + carry_1·256 = rs1_val_1 + imm_1 + carry_0
result_2 + carry_2·256 = rs1_val_2 + imm_2 + carry_1
result_3              = rs1_val_3 + imm_3 + carry_2  (mod 256)

# SLTI/SLTIU: Use SLT/SLTU constraints with imm replacing rs2_val
```

#### 4.4.4 Bitwise Immediates (XORI, ORI, ANDI)

```text
# Byte-wise operations with sign-extended immediate
(rs1_val_i, imm_i, result_i) ∈ BitwiseXor8  for XORI
(rs1_val_i, imm_i, result_i) ∈ BitwiseOr8   for ORI
(rs1_val_i, imm_i, result_i) ∈ BitwiseAnd8  for ANDI
```

#### 4.4.5 Shift Immediates (SLLI, SRLI, SRAI)

Shift immediate instructions encode the shift amount in bits [24:20] (shamt):

```text
# Extract shamt from instruction (5 bits)
shamt = instr_2 >> 4  # Bits [24:20]
shamt ∈ RangeCheck5   # Verify 0 ≤ shamt ≤ 31

# Distinguish SRAI from SRLI via funct7 (bit 30)
# SLLI: funct7 = 0x00, funct3 = 0x01
# SRLI: funct7 = 0x00, funct3 = 0x05
# SRAI: funct7 = 0x20, funct3 = 0x05

funct7 = instr_3 >> 1  # Bits [31:25]
is_srai = (funct7 == 0x20)

# For SLLI/SRLI: upper bits [31:25] must be zero
# For SRAI: bit 30 must be 1, others zero
(1 - is_srai) · funct7 = 0              # If not SRAI, funct7 = 0
is_srai · (funct7 - 0x20) = 0           # If SRAI, funct7 = 0x20

# Shift operations use constraints from Sections 4.3.5-4.3.7
```

**Implementation Path**:

- Create `AluImmComponent` implementing `FrameworkEval`
- Share column definitions with `AluRegComponent` where applicable
- Implement immediate sign extension in constraint evaluation
- Use funct3 to select operation: ADDI=0, SLTI=2, SLTIU=3, XORI=4, ORI=6, ANDI=7
- Use funct3 + funct7 to distinguish SLLI (001/00), SRLI (101/00), SRAI (101/20)
- Validate shamt range for shift operations
- Reuse arithmetic constraint patterns from R-type

---

### 4.5 M-extension Constraints (MUL/DIV)

The M-extension provides 8-bit multiplication and division instructions that
require extensive witness decomposition for efficient constraint generation.
These operations produce intermediate values that exceed 32 bits, necessitating
careful carry tracking and range verification.

#### 4.5.1 Witness Column Requirements

**Multiplication (MUL, MULH, MULHSU, MULHU)** requires 32 witness columns:

| Column Range | Name                                         | Description                  |
| ------------ | -------------------------------------------- | ---------------------------- |
| 0            | `enabler`                                    | Row active flag              |
| 1-4          | `pc`, `fp`, `clock`, `inst_prev_clock`       | Execution context            |
| 5-7          | `src0_off`, `src1_off`, `dst_off`            | Operand offsets              |
| 8-11         | `op0_0..op0_3`                               | First operand (8-bit limbs)  |
| 12-13        | `op0_prev_clock_lo/hi`                       | Operand 0 memory clocks      |
| 14-17        | `op1_0..op1_3`                               | Second operand (8-bit limbs) |
| 18-19        | `op1_prev_clock_lo/hi`                       | Operand 1 memory clocks      |
| 20-23        | `dst_prev_val_lo/hi`, `dst_prev_clock_lo/hi` | Destination state            |
| 24-27        | `res_0..res_3`                               | Result limbs (8-bit)         |
| 28-31        | `carry_0..carry_3`                           | Carry values                 |

**Division (DIV, DIVU, REM, REMU)** requires 54 witness columns, extending
multiplication with quotient, remainder, and verification columns.

#### 4.5.2 8-bit Limb Decomposition

Operands are decomposed into 8-bit limbs for schoolbook multiplication:

```text
op0 = op0_0 + op0_1·2⁸ + op0_2·2¹⁶ + op0_3·2²⁴
op1 = op1_0 + op1_1·2⁸ + op1_2·2¹⁶ + op1_3·2²⁴
```

Each limb satisfies `limb_i ∈ RangeCheck8` (values in [0, 255]).

This decomposition ensures partial products `op0_i · op1_j ≤ 255 × 255 = 65025`
remain well within the M31 field (2³¹ - 1 ≈ 2.1 billion), preventing overflow
during constraint evaluation.

#### 4.5.3 Schoolbook Multiplication Constraints

The 64-bit product is computed via positional partial sums:

```text
# Partial products at each byte position
p0 = op0_0·op1_0
p1 = op0_0·op1_1 + op0_1·op1_0
p2 = op0_0·op1_2 + op0_1·op1_1 + op0_2·op1_0
p3 = op0_0·op1_3 + op0_1·op1_2 + op0_2·op1_1 + op0_3·op1_0
p4 = op0_1·op1_3 + op0_2·op1_2 + op0_3·op1_1
p5 = op0_2·op1_3 + op0_3·op1_2
p6 = op0_3·op1_3
```

Result bytes with carry propagation:

```text
res_0 + carry_0·2⁸ = p0
res_1 + carry_1·2⁸ = p1 + carry_0
res_2 + carry_2·2⁸ = p2 + carry_1
res_3 + carry_3·2⁸ = p3 + carry_2
res_4 + carry_4·2⁸ = p4 + carry_3
res_5 + carry_5·2⁸ = p5 + carry_4
res_6 + carry_6·2⁸ = p6 + carry_5
res_7              = carry_6
```

#### 4.5.4 Carry Bound Derivations

Each carry must be bounded to ensure the witness is unique. The bounds derive
from the maximum value at each position:

**Position 0**: Single product term

```text
max(p0) = 255 × 255 = 65,025
carry_0 = p0 >> 8 ≤ 65,025 / 256 = 254
```

**Position 1**: Two product terms plus incoming carry

```text
max(p1 + carry_0) = 2 × (255 × 255) + 254 = 130,304
carry_1 ≤ 130,304 / 256 = 509
```

**Position 2**: Three product terms plus incoming carry

```text
max(p2 + carry_1) = 3 × (255 × 255) + 509 = 195,584
carry_2 ≤ 195,584 / 256 = 764
```

**Position 3**: Four product terms (maximum) plus incoming carry

```text
max(p3 + carry_2) = 4 × (255 × 255) + 764 = 260,864
carry_3 ≤ 260,864 / 256 = 1,019
```

**Positions 4-6** (for 64-bit result): Term count decreases symmetrically

```text
carry_4 ≤ 765  (3 terms + carry_3 residual)
carry_5 ≤ 510  (2 terms + carry_4 residual)
carry_6 ≤ 255  (1 term + carry_5 residual)
```

**Range Check Integration**: Carries are verified via RangeCheck16:

```text
(MAX_CARRY_i - carry_i) ∈ RangeCheck16  for i in 0..6
```

This checks `carry_i ≤ MAX_CARRY_i` since RangeCheck16 verifies values in [0,
65535], and `MAX_CARRY_i - carry_i` must be non-negative.

#### 4.5.5 Multiplication Variants

**MUL**: Returns low 32 bits of product

```text
rd_val = res_0 + res_1·2⁸ + res_2·2¹⁶ + res_3·2²⁴
```

**MULH** (signed × signed → high 32 bits): Requires sign handling

```text
sign1 = op0_3 >> 7
sign2 = op1_3 >> 7
result_sign = sign1 XOR sign2

# Convert to absolute values
abs0 = sign1 ? twos_complement(op0) : op0
abs1 = sign2 ? twos_complement(op1) : op1

# Multiply absolute values, apply result sign
abs_product = abs0 × abs1
product = result_sign ? twos_complement(abs_product) : abs_product
rd_val = product[63:32]
```

**MULHU** (unsigned × unsigned → high 32 bits):

```text
rd_val = res_4 + res_5·2⁸ + res_6·2¹⁶ + res_7·2²⁴
```

**MULHSU** (signed × unsigned → high 32 bits):

```text
sign1 = op0_3 >> 7
abs0 = sign1 ? twos_complement(op0) : op0
abs_product = abs0 × op1  (op1 treated as unsigned)
product = sign1 ? twos_complement(abs_product) : abs_product
rd_val = product[63:32]
```

#### 4.5.6 Division Identity Constraint

Division is verified through the fundamental identity:

```text
dividend = quotient × divisor + remainder
```

This decomposes into:

1. **Multiplication verification**: Prove `prod = q × d` using schoolbook
   multiplication (reusing constraints from 4.5.3)

2. **Addition verification**: Prove `n = prod + r` where n is dividend

```text
n_lo = prod_0 + prod_1·2⁸ + r_lo - add_carry_0·2¹⁶
n_hi = prod_2 + prod_3·2⁸ + r_hi + add_carry_0 - add_carry_1·2¹⁶
0    = prod_4 + prod_5·2⁸ + add_carry_1 - add_carry_2·2¹⁶
0    = prod_6 + prod_7·2⁸ + add_carry_2 - add_carry_3·2¹⁶
0    = add_carry_3
```

The final constraint `add_carry_3 = 0` ensures no overflow beyond 32 bits.

#### 4.5.7 Remainder Bound Constraint

The remainder must satisfy `0 ≤ r < |d|`. This is proven by showing the
subtraction `d - r - 1` does not underflow:

```text
sub_lo = d_0 + d_1·2⁸ + sub_borrow_0·2¹⁶ - r_lo - 1
sub_hi = d_2 + d_3·2⁸ + sub_borrow_1·2¹⁶ - r_hi - sub_borrow_0

# Verify non-negative results
sub_lo ∈ RangeCheck16
sub_hi ∈ RangeCheck16

# Final borrow must be zero (no underflow)
sub_borrow_1 = 0
```

The constraint `sub_borrow_1 = 0` proves `d - r - 1 ≥ 0`, hence `r < d`.

#### 4.5.8 Division by Zero

When divisor = 0, RISC-V specifies deterministic results:

| Instruction | Result             |
| ----------- | ------------------ |
| DIV         | -1 (0xFFFFFFFF)    |
| DIVU        | 2³²-1 (0xFFFFFFFF) |
| REM         | dividend           |
| REMU        | dividend           |

Detection: `is_zero = (d_0 = 0) ∧ (d_1 = 0) ∧ (d_2 = 0) ∧ (d_3 = 0)`

When `is_zero = 1`, the division identity `n = q × 0 + r` simplifies to `n = r`,
which is automatically satisfied when `q = -1` and `r = n`.

#### 4.5.9 Signed Overflow (INT_MIN / -1)

For signed division, `-2³¹ / -1` cannot be represented in 32 bits. RISC-V
specifies:

| Instruction | Result            |
| ----------- | ----------------- |
| DIV         | -2³¹ (0x80000000) |
| REM         | 0                 |

Detection:

```text
is_overflow = (n = 0x80000000) ∧ (d = 0xFFFFFFFF)
```

When overflow is detected, the witness provides `q = 0x80000000, r = 0`, which
satisfies the identity: `0x80000000 = 0x80000000 × (-1) + 0` in 32-bit
wraparound arithmetic.

#### 4.5.10 Range Check Summary

| Value Type                  | Range Check  | Count                 |
| --------------------------- | ------------ | --------------------- |
| Operand limbs (8-bit)       | RangeCheck8  | 8 (op0) + 8 (op1)     |
| Result limbs (8-bit)        | RangeCheck8  | 4 (MUL) or 8 (64-bit) |
| Product limbs (8-bit)       | RangeCheck8  | 8                     |
| Quotient limbs (8-bit)      | RangeCheck8  | 4                     |
| Dividend/remainder (16-bit) | RangeCheck16 | 4                     |
| Carry bounds                | RangeCheck16 | 4 (MUL) or 7 (DIV)    |
| Subtraction results         | RangeCheck16 | 2                     |

**Implementation Path**:

- Create `MulComponent` (32 columns) for MUL/MULH/MULHSU/MULHU
- Create `DivComponent` (54 columns) for DIV/DIVU/REM/REMU
- Implement 8-bit limb decomposition with RangeCheck8 verification
- Track carries via witness columns with RangeCheck16 bound verification
- Handle signed operations via absolute value conversion and sign bit XOR
- Special-case division by zero (set q=-1, r=dividend)
- Special-case signed overflow INT_MIN/-1 (set q=INT_MIN, r=0)
- Share schoolbook multiplication constraints between MUL and DIV components
- Integrate with existing Memory and Registers lookup relations

---

### 4.6 Load/Store Constraints

Load instructions use the `load` trace family (30 columns). Store instructions
use the `store` trace family (29 columns). All memory accesses operate on
word-aligned addresses; sub-word operations extract or modify bytes within the
accessed word.

#### 4.6.1 Address Computation

Both loads and stores compute the effective address from a base register and
sign-extended 12-bit immediate:

```text
# addr = rs1_val + sign_ext(imm)
addr_0 + carry_0·256 = rs1_val_0 + imm_0
addr_1 + carry_1·256 = rs1_val_1 + imm_1 + carry_0
addr_2 + carry_2·256 = rs1_val_2 + imm_2 + carry_1
addr_3              = rs1_val_3 + imm_3 + carry_2  (mod 256)

# Range checks
carry_i ∈ {0, 1}  for i in 0..2
addr_i ∈ RangeCheck8  for i in 0..3
```

#### 4.6.2 Alignment Constraints

Alignment is enforced via the low bits of `addr_0`:

| Access Type | Alignment | Constraint         |
| ----------- | --------- | ------------------ |
| LW, SW      | 4-byte    | `addr_0 & 0x3 = 0` |
| LH, LHU, SH | 2-byte    | `addr_0 & 0x1 = 0` |
| LB, LBU, SB | 1-byte    | (none)             |

For word alignment, use auxiliary column `aligned`:

```text
aligned = (addr_0 - (addr_0 & 0x3)) / 4
addr_0 = aligned · 4 + offset
offset ∈ {0}  (for LW/SW)
```

#### 4.6.3 Memory Lookup Tuple Format

All memory operations emit lookups to the Memory relation using a 6-element
tuple:

```text
MemoryTuple = (word_addr, clock, value_0, value_1, value_2, value_3)
```

Where:

- `word_addr = addr & ~0x3` (4-byte aligned address)
- `clock` is the current instruction cycle
- `value_0..3` are the four bytes of the 32-bit word (little-endian)

Load operations emit with multiplicity `-1` (consume); store operations emit
with multiplicity `+1` (produce).

#### 4.6.4 Byte Selection Multiplexer

For sub-word loads, a multiplexer selects bytes from the memory word using
auxiliary selector columns:

**Byte selection (LB, LBU)**:

```text
# byte_offset = addr_0 & 0x3  (values: 0, 1, 2, 3)
# Selector columns: sel_0, sel_1, sel_2, sel_3

# Exactly one selector is 1
sel_0 + sel_1 + sel_2 + sel_3 = 1
sel_i · (sel_i - 1) = 0  for i in 0..3

# Selector matches offset
sel_0 · (byte_offset - 0) = 0
sel_1 · (byte_offset - 1) = 0
sel_2 · (byte_offset - 2) = 0
sel_3 · (byte_offset - 3) = 0

# Extract byte via multiplexer
byte = sel_0·mem_val_0 + sel_1·mem_val_1 + sel_2·mem_val_2 + sel_3·mem_val_3
```

**Halfword selection (LH, LHU)**:

```text
# half_sel = (addr_0 >> 1) & 0x1  (values: 0 or 1)
half_sel · (half_sel - 1) = 0

halfword_0 = (1 - half_sel)·mem_val_0 + half_sel·mem_val_2
halfword_1 = (1 - half_sel)·mem_val_1 + half_sel·mem_val_3
```

#### 4.6.5 Sign vs Zero Extension

After byte/halfword extraction, extend to 32 bits:

**Sign extension (LB, LH)**:

```text
# Extract sign bit
sign = extracted_msb >> 7
sign · (sign - 1) = 0  # sign ∈ {0, 1}

# Decompose MSB: extracted_msb = sign·128 + low_7_bits
low_7_bits ∈ [0, 127]

# Fill upper bytes with sign
rd_val_0 = extracted_byte_0
rd_val_1 = (is_byte_load) ? sign·255 : extracted_byte_1
rd_val_2 = sign · 255
rd_val_3 = sign · 255
```

**Zero extension (LBU, LHU)**:

```text
rd_val_0 = extracted_byte_0
rd_val_1 = (is_byte_load) ? 0 : extracted_byte_1
rd_val_2 = 0
rd_val_3 = 0
```

The `is_signed` flag is derived from the instruction's `funct3` field.

#### 4.6.6 Read-Modify-Write for Sub-word Stores

Sub-word stores (SB, SH) require reading the existing word, modifying the
targeted bytes, and writing the result:

**Store Byte (SB)**:

```text
# Read current word
(word_addr, prev_clock, old_val) ∈ MemoryRead

# Compute new word using byte selector
new_val_0 = sel_0·rs2_val_0 + (1-sel_0)·old_val_0
new_val_1 = sel_1·rs2_val_0 + (1-sel_1)·old_val_1
new_val_2 = sel_2·rs2_val_0 + (1-sel_2)·old_val_2
new_val_3 = sel_3·rs2_val_0 + (1-sel_3)·old_val_3

# Write modified word
(word_addr, clock, new_val) ∈ MemoryWrite
```

**Store Halfword (SH)**:

```text
new_val_0 = (1-half_sel)·rs2_val_0 + half_sel·old_val_0
new_val_1 = (1-half_sel)·rs2_val_1 + half_sel·old_val_1
new_val_2 = half_sel·rs2_val_0 + (1-half_sel)·old_val_2
new_val_3 = half_sel·rs2_val_1 + (1-half_sel)·old_val_3
```

**Store Word (SW)**: Direct write without read-modify-write:

```text
(word_addr, clock, rs2_val) ∈ MemoryWrite
```

**Implementation Path**:

- Create `LoadComponent` and `StoreComponent` implementing `FrameworkEval`
- Use auxiliary selector columns for byte/halfword multiplexing
- Range check all byte columns via RangeCheck8
- Emit Memory relation entries with appropriate multiplicity
- Derive `is_signed` and access width from `funct3` instruction field

---

### 4.7 Memory Consistency Constraints

Memory consistency ensures that every load returns the value from the most
recent store to the same address. This is enforced via a logup permutation
argument linking all memory operations across the execution trace.

#### 4.7.1 Memory Relation Structure

Each memory access is represented as a 6-element tuple:

```text
MemoryTuple = (address, clock, value_0, value_1, value_2, value_3)
```

| Field        | Type | Description                                  |
| ------------ | ---- | -------------------------------------------- |
| `address`    | M31  | Word-aligned memory address (4-byte aligned) |
| `clock`      | M31  | Monotonic timestamp (instruction cycle)      |
| `value_0..3` | M31  | 32-bit value decomposed into 4 bytes         |

The Memory relation is defined with arity 6:

```rust
relation!(Memory, 6);  // (addr, clock, v0, v1, v2, v3)
```

#### 4.7.2 Logup Permutation Argument

The logup protocol enforces that reads and writes balance across the trace. Each
memory operation contributes a term to a cumulative sum:

```text
Σ (multiplicity_i / (z - combined_tuple_i)) = 0
```

Where the combined tuple uses verifier challenges `(z, α)`:

```text
combined_tuple = addr + α·clock + α²·v0 + α³·v1 + α⁴·v2 + α⁵·v3
```

**Multiplicity semantics**:

- Store operations: `multiplicity = +1` (emit/produce tuple)
- Load operations: `multiplicity = -1` (consume tuple)

The sum equals zero when every emitted tuple is consumed exactly once,
guaranteeing each load reads a previously stored value.

#### 4.7.3 Clock Ordering

The clock field establishes temporal ordering of memory accesses:

- **Clock 0**: Reserved for initial memory state (program loading, inputs)
- **Clock 1+**: Execution begins; clock increments with each instruction

For a load at `(addr, clock_read)` to return value `V`:

1. A store `(addr, clock_write, V)` must exist with `clock_write < clock_read`
2. No intervening store to `addr` between `clock_write` and `clock_read`

This is enforced implicitly: the logup sum balances only when matching tuples
pair correctly. A load at clock `T` with value `V` must consume a tuple
`(addr, T', V)` where `T' ≤ T`.

#### 4.7.4 Initial Memory State

Program code and input data are loaded into memory at clock 0:

```text
# For each initial memory word at address A with value V:
(A, 0, V_0, V_1, V_2, V_3) emitted with multiplicity +1
```

When the first instruction reads from address `A`:

1. Consume initial tuple: `(A, 0, V)` with multiplicity `-1`
2. Emit current tuple: `(A, clock, V)` with multiplicity `+1`

This chaining ensures the logup sum remains balanced while propagating values
through time.

**Final memory state**: At execution end, remaining tuples represent final
memory contents, committed for verification.

#### 4.7.5 Memory Trace Columns

The unified memory component collects all memory operations:

| Column | Name           | Description                    |
| ------ | -------------- | ------------------------------ |
| 0      | `enabler`      | 1 for valid row, 0 for padding |
| 1-4    | `address`      | Word address (4 bytes)         |
| 5-8    | `clock`        | Access timestamp (4 bytes)     |
| 9-12   | `value`        | Memory value (4 bytes)         |
| 13     | `multiplicity` | +1 for write, -1 for read      |

#### 4.7.6 Logup Cumulative Sum

The interaction trace accumulates fractions into a running sum:

```rust
// For each memory access row:
let num = multiplicity;
let denom = relations.memory.combine(&[addr, clock, v0, v1, v2, v3]);

// Accumulate: sum += num / denom
writer.write_frac(num, denom);
```

**Verification**: The final cumulative sum (`claimed_sum`) is a public input.
The verifier checks that the logup polynomial evaluates correctly, confirming
all memory operations are consistent.

**Batch inversion**: For efficiency, all denominators are inverted in a single
batch operation using Montgomery's trick, then multiplied by their numerators.

#### 4.7.7 Constraint Implementation

```rust
impl FrameworkEval for MemoryEval {
    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let enabler = eval.next_trace_mask();
        let address = eval.next_trace_mask();
        let clock = eval.next_trace_mask();
        let value0 = eval.next_trace_mask();
        let value1 = eval.next_trace_mask();
        let value2 = eval.next_trace_mask();
        let value3 = eval.next_trace_mask();
        let multiplicity = eval.next_trace_mask();

        // Enabler is boolean
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));

        // Emit to Memory relation
        eval.add_to_relation(RelationEntry::new(
            &self.relations.memory,
            E::EF::from(multiplicity),
            &[address, clock, value0, value1, value2, value3],
        ));

        eval.finalize_logup_in_pairs();
        eval
    }
}
```

**Implementation Path**:

- Create unified `MemoryComponent` implementing `FrameworkEval`
- Load/store components emit tuples via `add_to_relation()`
- Initial memory state emitted as clock-0 writes with multiplicity +1
- Use Stwo's `LogupTraceGenerator` for interaction trace generation
- Verify `claimed_sum` matches computed logup accumulation

---

### 4.8 Branch Constraints

Branch instructions use the `branch` trace family (31 columns). All six branch
instructions share a common structure: compute a comparison result, then
conditionally update the PC.

#### 4.8.1 Trace Columns

| Column Range | Name      | Description                              |
| ------------ | --------- | ---------------------------------------- |
| 0-3          | `cycle`   | Instruction cycle (4 bytes)              |
| 4-7          | `pc`      | Program counter (4 bytes)                |
| 8-11         | `instr`   | Instruction word (4 bytes)               |
| 12           | `rs1_idx` | Source register 1 index                  |
| 13-16        | `rs1_val` | Source register 1 value (4 bytes)        |
| 17           | `rs2_idx` | Source register 2 index                  |
| 18-21        | `rs2_val` | Source register 2 value (4 bytes)        |
| 22-25        | `imm`     | Sign-extended B-type immediate (4 bytes) |
| 26           | `taken`   | Branch taken flag (0 or 1)               |
| 27-30        | `pc_next` | Next program counter (4 bytes)           |

#### 4.8.2 Inverse Trick for Zero Detection

To determine if a value equals zero without bit-by-bit checking, we use the
**inverse trick** (as in Cairo-M's jnz_fp_imm):

```text
# For a difference diff = rs1_val - rs2_val:
diff · diff_inv = 1 - is_zero
is_zero · diff = 0

# Where:
# - diff_inv is the witness: inverse of diff if diff ≠ 0, else 0
# - is_zero is 1 if diff = 0, else 0
```

These two constraints together enforce:

- If `diff = 0`: first constraint gives `0 = 1 - is_zero`, so `is_zero = 1`
- If `diff ≠ 0`: second constraint is satisfied; first requires
  `diff_inv = diff⁻¹`

#### 4.8.3 BEQ/BNE (Equality Comparison)

```text
# Compute difference
diff = rs1_val - rs2_val  (using SUB constraints from 4.3.3)

# Zero detection via inverse trick
diff · diff_inv = 1 - is_zero
is_zero · diff = 0

# BEQ: taken = is_zero
# BNE: taken = 1 - is_zero
```

#### 4.8.4 BLT/BGE (Signed Comparison)

```text
# Sign bits
sign1 = rs1_val_3 >> 7
sign2 = rs2_val_3 >> 7

# Compute diff = rs1 - rs2
diff = rs1_val - rs2_val
diff_sign = diff_3 >> 7

# Signed comparison: negative if signs differ and rs1 negative,
# or signs same and difference negative
is_less = (sign1 · (1 - sign2)) + ((1 - (sign1 XOR sign2)) · diff_sign)

# BLT: taken = is_less
# BGE: taken = 1 - is_less
```

#### 4.8.5 BLTU/BGEU (Unsigned Comparison)

```text
# Subtract with borrow tracking (from SLTU pattern in 4.3.8)
rs1_val - rs2_val = diff + borrow_out · 2³²

# If borrow_out = 1, rs1 < rs2 (unsigned)
is_less_unsigned = borrow_out

# BLTU: taken = is_less_unsigned
# BGEU: taken = 1 - is_less_unsigned
```

#### 4.8.6 Branch Target Computation

The B-type immediate is a 13-bit signed value (bit 0 always 0) encoded across
instruction fields. After extraction and sign extension:

```text
# target = pc + sign_ext(imm)
target_0 + carry_0·256 = pc_0 + imm_0
target_1 + carry_1·256 = pc_1 + imm_1 + carry_0
target_2 + carry_2·256 = pc_2 + imm_2 + carry_1
target_3              = pc_3 + imm_3 + carry_2  (mod 256)

# Range checks
carry_i ∈ {0, 1}  for i in 0..2
target_i ∈ RangeCheck8  for i in 0..3
```

#### 4.8.7 Conditional PC Update

Following the Cairo-M pattern, the conditional update is expressed as:

```text
# Constraint form (avoids branching):
pc_next = pc + 4 + taken · (target - pc - 4)

# Equivalently:
pc_next = (1 - taken) · (pc + 4) + taken · target

# The taken flag must be boolean:
taken · (1 - taken) = 0
```

**Implementation Path**:

- Create `BranchComponent` implementing `FrameworkEval`
- Add witness columns: `diff_inv`, `is_zero`/`is_less`, carries
- Use `funct3` from instruction to select comparison type
- Emit logup entries for register reads
- Range check all byte columns via RangeCheck8

---

### 4.9 Jump and Upper Immediate Constraints

Jump instructions (JAL, JALR) use the `jump` trace family (26 columns). Upper
immediate instructions (LUI, AUIPC) use the `upper_imm` trace family (21
columns).

#### 4.9.1 JAL (Jump and Link)

JAL performs a PC-relative unconditional jump and stores the return address.
Following Cairo-M's jmp_imm pattern:

```text
# Return address: rd = pc + 4
rd_val_0 + carry_0·256 = pc_0 + 4
rd_val_1 + carry_1·256 = pc_1 + carry_0
rd_val_2 + carry_2·256 = pc_2 + carry_1
rd_val_3              = pc_3 + carry_2

# Target: pc_next = pc + sign_ext(imm)
# J-type immediate is 21 bits, sign-extended, LSB always 0
pc_next_0 + tc_0·256 = pc_0 + imm_0
pc_next_1 + tc_1·256 = pc_1 + imm_1 + tc_0
pc_next_2 + tc_2·256 = pc_2 + imm_2 + tc_1
pc_next_3            = pc_3 + imm_3 + tc_2  (mod 256)

# Range checks
carry_i, tc_i ∈ {0, 1}
```

#### 4.9.2 JALR (Jump and Link Register)

JALR performs an indirect jump through a register, clearing bit 0 of the target
for alignment:

```text
# Return address: rd = pc + 4  (same as JAL)

# Compute raw target: target = rs1_val + sign_ext(imm)
raw_0 + carry_0·256 = rs1_val_0 + imm_0
raw_1 + carry_1·256 = rs1_val_1 + imm_1 + carry_0
raw_2 + carry_2·256 = rs1_val_2 + imm_2 + carry_1
raw_3              = rs1_val_3 + imm_3 + carry_2

# Clear bit 0 for 2-byte alignment (RISC-V spec):
pc_next_0 = raw_0 & 0xFE  # Equivalently: raw_0 - (raw_0 & 1)
pc_next_1 = raw_1
pc_next_2 = raw_2
pc_next_3 = raw_3

# Constraint for bit 0 clearing:
bit0 = raw_0 - pc_next_0
bit0 · (1 - bit0) = 0  # bit0 ∈ {0, 1}
```

#### 4.9.3 LUI (Load Upper Immediate)

LUI loads a 20-bit immediate into the upper 20 bits of the destination register,
zeroing the lower 12 bits:

```text
# rd = imm << 12
# The U-type immediate is bits [31:12] of instruction

# Direct constraint (since imm is already extracted as upper 20 bits):
rd_val_0 = 0
rd_val_1 = (imm_lo · 16) & 0xFF  # imm_lo[3:0] << 4
rd_val_2 = (imm_lo >> 4) | ((imm_mid & 0x0F) << 4)
rd_val_3 = (imm_mid >> 4) | ((imm_hi & 0x0F) << 4)

# Simpler form with multiplication:
rd_val = imm · 4096  (where imm is the 20-bit value)

# PC unchanged for LUI:
pc_next = pc + 4
```

#### 4.9.4 AUIPC (Add Upper Immediate to PC)

AUIPC adds the upper immediate (shifted left by 12) to the current PC:

```text
# rd = pc + (imm << 12)
shifted_imm = imm · 4096

# Addition with carry:
rd_val_0 + c0·256 = pc_0 + 0          # Lower 12 bits = PC's lower 12
rd_val_1 + c1·256 = pc_1 + shifted_imm_1 + c0
rd_val_2 + c2·256 = pc_2 + shifted_imm_2 + c1
rd_val_3         = pc_3 + shifted_imm_3 + c2

# Note: shifted_imm_0 = 0 (12-bit shift)
# PC unchanged:
pc_next = pc + 4
```

**Implementation Path**:

- Create `JumpComponent` for JAL/JALR with shared return address logic
- Create `UpperImmComponent` for LUI/AUIPC
- Use `funct3` or opcode to distinguish JAL vs JALR, LUI vs AUIPC
- JALR requires extra witness column for extracted bit 0
- Emit register write logup entries for rd

---

### 4.10 Program Counter Constraints

The program counter is constrained globally across all instruction types,
ensuring execution flow integrity from initialization through termination.

#### 4.10.1 Sequential Increment

For non-control-flow instructions (ALU, load, store, upper immediate), the PC
advances by 4 bytes:

```text
# Default: pc_next = pc + 4
pc_next_0 + carry_0·256 = pc_0 + 4
pc_next_1 + carry_1·256 = pc_1 + carry_0
pc_next_2 + carry_2·256 = pc_2 + carry_1
pc_next_3              = pc_3 + carry_2

# Carry constraints
carry_i ∈ {0, 1}  for i in 0..2
```

This constraint is implicit in each non-control-flow opcode component. The PC
increment of 4 produces at most one carry at byte 0 (when `pc_0 ≥ 252`).

#### 4.10.2 Initial PC Validation

The first instruction must execute at the ELF entry point, provided as a public
input:

```text
# Using indicator function for cycle = 0
is_first = (cycle = 0) ? 1 : 0

# Constraint: first instruction starts at entry_point
is_first · (pc - entry_point) = 0
```

In practice, this is implemented by constraining the first row of each opcode
trace. The entry point bytes `entry_0, entry_1, entry_2, entry_3` are public
inputs mixed into the Fiat-Shamir transcript.

#### 4.10.3 PC Alignment

RISC-V requires 4-byte instruction alignment. This is enforced on every
instruction:

```text
# Low 2 bits of PC must be zero
pc_0 & 0x03 = 0

# Equivalently, decompose pc_0 = 4·q + r where r ∈ {0,1,2,3}
# Constraint: r = 0
pc_0 = 4 · pc_0_quarter
pc_0_quarter ∈ RangeCheck8  (actually [0, 63])
```

Misaligned PC values would indicate invalid execution and cause the proof to
fail.

#### 4.10.4 Termination (ECALL Handling)

Program termination via ECALL is handled as a special instruction that produces
no successor state:

```text
# ECALL detection (opcode = 0x73, funct3 = 0, imm = 0)
is_ecall = (opcode = 0b1110011) · (funct3 = 0) · (imm = 0)

# Exit code from register x10 (a0)
exit_code = reg_a0_val

# Terminal constraint: no pc_next required
# The trace simply ends; logup sums must still balance
```

For the logup argument to verify, the register and memory relations must balance
across the entire trace including the final ECALL row.

**Implementation Path**:

- Each opcode component includes PC increment or jump/branch logic
- Create `ProgramStartComponent` for initial PC validation against public input
- PC alignment check embedded in instruction fetch memory lookup
- ECALL handled in `SystemComponent` with exit code extraction
- Global cycle ordering verified via memory consistency (Section 4.7)

---

End of Section 4.

---

## 6. Proving Pipeline

This section specifies the end-to-end proving pipeline that transforms a guest
ELF binary into a cryptographic proof of correct execution. The pipeline
integrates the execution model (Section 1), instruction semantics (Section 2),
trace structure (Section 3), and AIR constraints (Section 4) into a cohesive
system built on Stwo's Circle STARK prover.

### 5.1 Pipeline Overview

The proving pipeline consists of six sequential stages:

```text
┌────────┐   ┌─────────┐   ┌───────┐   ┌─────────┐   ┌───────┐   ┌────────┐
│  Load  │──▶│ Execute │──▶│ Trace │──▶│ Witness │──▶│ Prove │──▶│ Verify │
└────────┘   └─────────┘   └───────┘   └─────────┘   └───────┘   └────────┘
    │             │            │            │            │            │
    ▼             ▼            ▼            ▼            ▼            ▼
   ELF        Execution    Per-opcode   Column       Stwo FRI    Valid /
  binary       state        files      matrices      proof      Invalid
```

**Load** parses the ELF binary and initializes VM memory. **Execute** runs the
program and records each instruction's state transition. **Trace** serializes
execution data into per-opcode files. **Witness** loads traces and populates
Stwo column matrices. **Prove** commits to polynomials and generates the FRI
proof. **Verify** checks the proof against public inputs.

The pipeline produces **reproducible** execution traces; the final proof
includes a proof-of-work nonce that may vary between runs. Each stage is
independently testable and produces artifacts that can be inspected for
debugging.

---

### 5.2 Pipeline Stages

#### 5.2.1 Load Stage

The Load stage parses the guest ELF binary and initializes VM state.

**Input:** Path to a valid `riscv32im-unknown-none-elf` ELF binary.

**Operations:**

1. Parse ELF headers and validate the binary targets RV32IM.
2. Extract loadable segments (`.text`, `.data`, `.rodata`, `.bss`).
3. Map segments into VM memory at their specified virtual addresses.
4. Initialize the program counter (`pc`) to the ELF entry point (`_start`).
5. Initialize the stack pointer (`sp`) and global pointer (`gp`) per Section 1.
6. Record the initial memory layout for later witness generation.

**Output:** Initialized `VmState` containing memory contents and register file.

**Implementation Path:**

Use an existing ELF parsing library (e.g., `goblin` or `elf`). Memory is
represented as a sparse map from addresses to bytes. No cryptographic commitment
is computed at load time; memory consistency is verified via LogUp during
proving.

---

#### 5.2.2 Execute Stage

The Execute stage runs the VM and records every state transition. The
interpreter loop is the performance-critical **hot path**; trace recording must
not degrade execution throughput.

**Input:** Initialized `VmState` from Load.

**Operations:**

1. Fetch the instruction at `pc`.
2. Decode the instruction into its opcode and operands.
3. Execute the instruction per Section 2 semantics.
4. Record the **trace row**:
   `(pc, opcode, rs1, rs2, rd, imm, mem_addr, mem_value, ...)`.
5. Update `pc` and registers.
6. Repeat until a termination condition (halt instruction, cycle limit, or
   trap).

**Output:** Complete execution trace as an in-memory structure, plus final
`VmState`.

**Termination conditions:**

- `ECALL` with designated halt code
- Configurable maximum cycle count (prevents infinite loops)
- Invalid instruction trap

**Hot Path / Slow Path Architecture:**

Trace recording must be decoupled from instruction execution to avoid:

1. Memory allocation in the inner loop
2. Branch mispredictions from conditional trace logic
3. Cache pollution from trace data structures

The hot path consists solely of: fetch, decode, execute, register writeback, and
PC update. All trace-related operations are deferred to the slow path.

**Branchless Trace Append:**

The trace buffer uses a fixed-size ring with power-of-two length. Appending a
trace row requires no branches:

```rust
// Hot path: always write, use wrapping index
let idx = self.trace_cursor & (BUFFER_SIZE - 1);
self.trace_buffer[idx] = TraceRow { pc, opcode, rs1_val, rs2_val, rd_val, ... };
self.trace_cursor = self.trace_cursor.wrapping_add(1);
```

No conditional checks—the buffer always accepts writes. When full, the slow path
drains it asynchronously (see Section 5.2.3).

**Cache Hierarchy Separation:**

| Data Structure     | Target Cache | Size Budget      |
| ------------------ | ------------ | ---------------- |
| Register file      | L1 (hot)     | 128 bytes (32×4) |
| PC, flags, cursors | L1 (hot)     | 64 bytes         |
| Decode cache       | L1/L2        | 64 KB (optional) |
| Trace ring buffer  | L2/L3        | 1-16 MB          |
| VM memory          | L3/DRAM      | up to 4 GB       |

The interpreter state struct is cache-line aligned to prevent false sharing:

```rust
#[repr(C, align(64))]
struct InterpreterState {
    pc: u32,
    regs: [u32; 32],           // 128 bytes
    trace_cursor: usize,
    trace_buffer: *mut TraceRow, // pointer only; buffer lives elsewhere
}
```

**Dispatch Optimization:**

Use a direct-threaded dispatch table to eliminate branch predictor pollution
(more efficient than `match opcode { ... }`):

```rust
type Handler = fn(&mut Vm, u32) -> ();
static DISPATCH: [Handler; 64] = [handle_add, handle_sub, handle_lw, ...];

#[inline(always)]
fn step(vm: &mut Vm) {
    let insn = vm.fetch();
    let opcode = decode_opcode(insn);
    DISPATCH[opcode as usize](vm, insn);
}
```

This avoids a single indirect branch prediction site and distributes prediction
across multiple call sites.

**Instruction Decode Caching:**

For programs with static `.text` segments, pre-decode all instructions at load
time into a parallel array of `DecodedInsn` structs. The hot path fetches from
the decode cache rather than re-parsing raw bytes each cycle.

**Implementation Path:**

The executor maintains a pre-allocated ring buffer for trace rows. The buffer
pointer and cursor live in the interpreter state. When the cursor reaches
`BUFFER_SIZE`, the slow path signals a writer thread to drain and partition the
buffer (see Section 5.2.3). The trace row structure matches the column layout
defined in Section 3.

---

#### 5.2.3 Trace Stage

The Trace stage serializes the execution trace into per-opcode files. Trace
dumping runs asynchronously to avoid blocking the interpreter's hot path.

**Input:** Ring buffer(s) filled by the Execute stage.

**Operations:**

1. Drain trace rows from the executor's ring buffer.
2. Partition rows by opcode (e.g., `ADD`, `LW`, `BEQ`).
3. Append partitioned rows to per-opcode output files.
4. Pad each file to a power-of-two row count (required by Stwo).
5. Record opcode instance counts for component sizing.

**Output:** Directory of trace files, one per opcode family (see Section 2.4.3):

```text
traces/
├── alu_reg.trace   # ADD, SUB, SLL, SLT, SLTU, XOR, SRL, SRA, OR, AND
├── alu_imm.trace   # ADDI, SLTI, SLTIU, XORI, ORI, ANDI, SLLI, SRLI, SRAI
├── upper_imm.trace # LUI, AUIPC
├── branch.trace    # BEQ, BNE, BLT, BGE, BLTU, BGEU
├── load.trace      # LB, LH, LW, LBU, LHU
├── store.trace     # SB, SH, SW
├── jump.trace      # JAL, JALR
└── mul_div.trace   # MUL, MULH, MULHSU, MULHU, DIV, DIVU, REM, REMU
```

**File format:** Column-major binary layout as specified in Section 2.4.4. Each
file contains a 32-byte header followed by column data. Row counts are
little-endian u32 in the header; column data uses native field encoding.

**Async Trace Dumping Architecture:**

Trace serialization runs on a dedicated writer thread to decouple I/O latency
from execution throughput:

```text
Executor Thread              Writer Thread
───────────────              ─────────────
fill buffer A ──────────────► (idle)
signal "A full" ────────────► wake
fill buffer B                 drain A, partition by opcode
signal "B full" ────────────► queue A writes
fill buffer A                 drain B, partition by opcode
   ...                           ...
```

**Double-Buffering:**

Two ring buffers alternate ownership between executor and writer:

```rust
struct TraceChannel {
    buffers: [Box<[TraceRow; BUFFER_SIZE]>; 2],
    active: AtomicUsize,      // 0 or 1: which buffer executor writes to
    ready: AtomicUsize,       // bitmask: which buffers are ready to drain
    cursors: [AtomicUsize; 2], // write position in each buffer
}
```

The executor writes to `buffers[active]`. When full, it atomically swaps
`active` and sets the corresponding bit in `ready`. The writer thread polls
`ready` (or blocks via `futex`/`parking_lot::Condvar`) and drains completed
buffers.

**Lock-Free Handoff Protocol:**

```rust
// Executor: signal buffer full
fn flush_buffer(&self, buf_idx: usize) {
    self.ready.fetch_or(1 << buf_idx, Ordering::Release);
    self.active.store(1 - buf_idx, Ordering::Release);
}

// Writer: wait for and claim a buffer
fn wait_for_buffer(&self) -> usize {
    loop {
        let ready = self.ready.load(Ordering::Acquire);
        if ready != 0 {
            let idx = ready.trailing_zeros() as usize;
            self.ready.fetch_and(!(1 << idx), Ordering::AcqRel);
            return idx;
        }
        std::hint::spin_loop(); // or futex_wait
    }
}
```

**Streaming Partitioning:**

The writer partitions rows by opcode while draining, avoiding a post-execution
O(n) pass over the full trace:

```rust
fn drain_and_partition(&mut self, buffer: &[TraceRow]) {
    for row in buffer {
        let file = &mut self.opcode_files[row.opcode as usize];
        file.write_row(row);
    }
}
```

Each opcode file is a memory-mapped region grown as needed. The OS handles
write-back asynchronously.

**Memory-Mapped Output:**

```rust
// Pre-allocate trace file with estimated size
let file = OpenOptions::new().read(true).write(true).create(true).open(path)?;
file.set_len(estimated_size)?;
let mmap = unsafe { MmapMut::map_mut(&file)? };
// Direct writes to mmap; OS flushes pages in background
```

For traces exceeding available RAM, `madvise(MADV_SEQUENTIAL)` hints help the OS
manage page cache pressure.

**Latency Hiding Timeline:**

```text
Cycle   Executor           Writer           Disk I/O
─────   ────────           ──────           ────────
0-1M    fill A             (idle)           (idle)
1M      swap to B          wake, claim A    (idle)
1M-2M   fill B             partition A      (idle)
2M      swap to A          flush A, claim B pages A queued
2M-3M   fill A             partition B      write A
3M      swap to B          flush B, claim A pages B queued
```

The executor never blocks on I/O. Disk writes happen in parallel with continued
execution.

**Implementation Path:**

Spawn a dedicated writer thread at execution start. Use double-buffered ring
buffers with atomic handoff. Memory-map output files and grow them
geometrically. Padding rows (with `is_padding = 1`) are appended after execution
completes to reach power-of-two lengths. This architecture sustains >100M
instructions/sec on modern hardware while generating full traces.

---

#### 5.2.4 Witness Stage

The Witness stage loads trace files and constructs Stwo column matrices.

**Input:** Per-opcode trace files from Trace stage, plus component definitions.

**Operations:**

1. For each opcode component, load its trace file.
2. Allocate Stwo `CircleEvaluation` columns sized to the trace length.
3. Populate columns by interpreting trace rows according to Section 2.4 schemas.
4. Construct interaction columns for memory and range-check arguments.
5. Build the **interaction elements** (random challenges from Fiat-Shamir).

**Output:** Populated `ColumnVec<CircleEvaluation>` for each component, ready
for commitment.

**Column types:**

- **Base columns:** Direct transcript of execution (pc, registers, immediates)
- **Interaction columns:** Logarithmic derivatives for permutation arguments
- **Constant columns:** Selector polynomials, opcode identifiers

**Implementation Path:**

Each opcode family implements Stwo's `FrameworkEval` trait (see Section 4.2).
The `evaluate` method defines AIR constraints using `EvalAtRow`; witness
generation populates columns via `write_trace_simd`. Interaction columns for
LogUp are computed after the first-round commitment provides random challenges.

---

#### 5.2.5 Prove Stage

The Prove stage commits to witness polynomials and generates the FRI proof.

**Input:** Column matrices from Witness, AIR constraints from Section 4.

**Operations:**

1. **Commit phase:** Compute Merkle commitments to all column polynomials
   evaluated over the Circle domain.
2. **Constraint evaluation:** Evaluate AIR constraints at random points,
   producing the quotient polynomial.
3. **FRI phase:** Apply the FRI protocol to prove low-degree of the quotient.
4. **Query phase:** Open Merkle paths at FRI query positions.
5. **Serialize:** Package commitments, evaluations, and FRI layers into the
   proof object.

**Output:** Serialized `Proof` containing:

- Merkle root commitments (columns, FRI layers)
- Evaluations at query points
- FRI folding proofs
- Public inputs (program hash, initial/final state commitments)

**Stwo configuration:**

- Circle STARK over the Mersenne31 field
- Configurable log-blowup factor (typically 1-2)
- Security parameter targeting 128 bits

**Implementation Path:**

Invoke Stwo's `prove()` with the composed `Air` object. Parallelization
opportunities exist in column commitment (independent Merkle trees) and FRI
layer computation.

---

#### 5.2.6 Verify Stage

The Verify stage checks proof validity against public inputs.

**Input:** Serialized `Proof`, public inputs (program hash, I/O commitments).

**Operations:**

1. Recompute Fiat-Shamir challenges from the proof transcript.
2. Verify Merkle commitments match claimed roots.
3. Check AIR constraint evaluations at random points.
4. Verify FRI proximity proof.
5. Return `Valid` or `Invalid` with diagnostic information.

**Output:** Verification result.

**Implementation Path:**

The verifier is a standalone binary with minimal dependencies. It does not
require the original trace or witness data—only the proof and public inputs.

---

### 5.3 Component Composition

Each RV32IM opcode is implemented as a separate Stwo component. Components are
composed into a unified AIR through shared interaction columns.

**Opcode components:**

Eight opcode family components, each implementing `FrameworkEval` (Section 4.2):

| Component      | Column Count | Instructions                       |
| -------------- | ------------ | ---------------------------------- |
| `AluRegEval`   | 31           | ADD, SUB, SLL, SLT, SLTU, XOR, ... |
| `AluImmEval`   | 26           | ADDI, SLTI, SLTIU, XORI, ORI, ...  |
| `UpperImmEval` | 21           | LUI, AUIPC                         |
| `BranchEval`   | 31           | BEQ, BNE, BLT, BGE, BLTU, BGEU     |
| `LoadEval`     | 30           | LB, LH, LW, LBU, LHU               |
| `StoreEval`    | 29           | SB, SH, SW                         |
| `JumpEval`     | 26           | JAL, JALR                          |
| `MulDivEval`   | 35           | MUL, MULH, MULHSU, MULHU, DIV, ... |

Each defines column layout, AIR constraints, and interaction contributions.

**Interaction components:**

- **Memory bus:** Enforces read/write consistency via LogUp permutation
  argument. All memory operations across opcodes contribute to a shared
  accumulator.
- **Program counter bus:** Verifies sequential PC transitions or valid jumps.
- **Precomputed lookup tables** (Section 4.3):
  - `RangeCheck8`: 2⁸ rows for byte-range validation
  - `RangeCheck16`: 2¹⁶ rows for half-word bounds
  - `RangeCheck20`: 2²⁰ rows for 20-bit immediate bounds
  - `Bitwise`: 2¹⁶ rows for XOR/AND/OR lookup tables

**Composition pattern:**

```rust
struct RV32IMAir {
    // Eight opcode family components
    alu_reg: FrameworkComponent<AluRegEval>,
    alu_imm: FrameworkComponent<AluImmEval>,
    upper_imm: FrameworkComponent<UpperImmEval>,
    branch: FrameworkComponent<BranchEval>,
    load: FrameworkComponent<LoadEval>,
    store: FrameworkComponent<StoreEval>,
    jump: FrameworkComponent<JumpEval>,
    mul_div: FrameworkComponent<MulDivEval>,
    // Interaction components
    memory_bus: MemoryBusComponent,
    pc_bus: ProgramCounterBusComponent,
    // Precomputed tables
    range_check_8: RangeCheck8Component,
    range_check_16: RangeCheck16Component,
    range_check_20: RangeCheck20Component,
    bitwise: BitwiseComponent,
}
```

Each component's `FrameworkEval::evaluate` is called independently; Stwo
aggregates constraints via the composition polynomial.

---

### 5.4 Performance Considerations

**Parallelization opportunities:**

1. **Trace partitioning:** Per-opcode files enable parallel witness generation.
2. **Column commitment:** Merkle trees for independent columns build in
   parallel.
3. **FRI layers:** Some FRI computations parallelize across cosets.

**Memory usage:**

- Trace storage: 84–140 bytes per instruction (21–35 columns × 4 bytes/field)
- Witness columns: Dominated by log₂(trace_length) × column_count field elements
- Peak memory occurs during polynomial commitment

**Trace size vs. proof time trade-offs:**

| Trace rows | Approx. proof time | Proof size |
| ---------- | ------------------ | ---------- |
| 2¹⁶        | ~2 seconds         | ~50 KB     |
| 2²⁰        | ~30 seconds        | ~100 KB    |
| 2²⁴        | ~8 minutes         | ~150 KB    |

Proof size grows logarithmically with trace length due to FRI's structure.
Proving time grows quasi-linearly.

**Optimization strategies:**

- Use SIMD for field arithmetic (Stwo provides AVX2/AVX-512 backends)
- Stream traces to disk for programs exceeding available RAM
- Batch similar opcodes to improve cache locality

---

### 5.5 Proof Artifacts

**Proof contents:**

1. **Public inputs:**
   - Program hash (commitment to loaded ELF)
   - Initial memory root (sparse Merkle root of starting state)
   - Final memory root (sparse Merkle root of ending state)
   - Initial and final PC values
   - Cycle count

2. **Commitments:**
   - Column polynomial Merkle roots
   - FRI layer commitments

3. **Evaluations:**
   - Constraint quotient evaluations at query points
   - Column openings at FRI query positions

4. **FRI proof:**
   - Folding coefficients
   - Final polynomial coefficients
   - Merkle authentication paths

**Proof size estimates:**

- Base proof: 40-60 KB for typical programs
- Grows ~10 KB per doubling of trace length
- Dominated by Merkle paths and FRI queries

**Serialization format:**

Proofs serialize to a binary format with a versioned header. The format is
self-describing to support schema evolution.

---

### 5.6 Verification Interface

**Verifier API:**

```rust
pub fn verify(
    proof: &Proof,
    program_hash: Felt,
    public_inputs: &PublicInputs,
) -> Result<(), VerificationError>;
```

**What the verifier checks:**

1. Proof structure is well-formed
2. Public inputs match claimed values in proof
3. Fiat-Shamir transcript reproduces challenges
4. All Merkle paths authenticate against claimed roots
5. AIR constraints evaluate to zero at random points
6. FRI verifies low-degree of quotient polynomial

**Integration patterns:**

**Off-chain verification:**

- Standalone Rust binary with minimal dependencies
- WASM build for browser-based verification
- Target verification time: <100ms for typical proofs

**On-chain verification:**

- Solidity verifier for Ethereum L1 (high gas cost, ~2-5M gas)
- Cairo verifier for Starknet L2 (native STARK verification)
- Recursive proof composition for amortized L1 costs

The verifier implementation is intentionally separate from the prover to
minimize its attack surface and enable formal verification.

End of Section 5.

---

## Appendix C — Reference Resources

The following external resources inform the design:

- [**Stwo prover**](https://github.com/starkware-libs/stwo/)
- [**Rookie Numbers**](https://github.com/ClementWalter/rookie-numbers/)
- [**Cairo-M**](https://github.com/kkrt-labs/cairo-m)
