# AIRS

This document mirrors the structure of `../stark-v/airs.md` and enumerates the
AIRs that implement OpenVM's RV32IM extension. All register values, immediates,
and memory words are decomposed into `RV32_REGISTER_NUM_LIMBS = 4` little-endian
limbs of `RV32_CELL_BITS = 8` bits. The cores communicate with helper buses such
as the bitwise lookup, variable range checker, and range tuple checker exactly
as in the source files under `extensions/rv32im/circuit/src`.

## 0. Conventions

### 0.1 Register limbs and helper lookups

- Every register limb `x_i` is implicitly constrained to `[0, 2^8)` either via
  boolean carries or via `BitwiseOperationLookupBus::send_range`.
- Multi-limb additions and multiplications introduce temporary carries. They are
  normalized by dividing by `2^RV32_CELL_BITS`; when asserted boolean, they
  prove the target limb lies in range.
- The bitwise lookup bus is reused both to verify XOR/OR/AND/SR\* operations and
  to range-check limbs when no bitwise opcode is active.
- The variable range checker bus constrains auxiliary values such as shift
  carries and program-counter fragments to the precise bit width advertised in
  the code.
- When a tuple of values must simultaneously lie in specified ranges (for
  example in the multiplication and division cores), the `RangeTupleCheckerBus`
  enforces that tuple through a single lookup.

### 0.2 Adapter responsibilities

The adapters defined in `extensions/rv32im/circuit/src/adapters` interface every
core with the program and memory buses. The following invariants hold for all
instructions:

- **Program fetch and clocking.** The `ExecutionBridge` verifies that the
  instruction opcode and operands match what is stored in the program segment at
  `from_pc`, and either increments the program counter by 4 or sets it to the
  `to_pc` supplied by the core.
- **Register access.** Reads and writes go through the register address space
  (`RV32_REGISTER_AS`). Writes to `x0` are suppressed by the adapter;
  loads/stores use the `f` bit to record whether a write-back must occur.
- **Immediates.** For ALU, branch, and JALR instructions the adapter either
  feeds limbs read from `rs2` or injects the sign-extended 16-bit immediate. The
  adapter constrains the selector `rs2_as` and range-checks the injected limbs.
- **Memory operations.** The load/store adapter enforces that `mem_as` is in
  `{0, 1, 2}` for loads and `{2, 3, 4}` for stores, reads aligned pointers, and
  enforces that the byte/half-word alignment information passed to the core
  matches the offset used to access memory segment `RV32_MEMORY_AS`.
- **Branch/JAL adapters.** Branch adapters supply `from_pc` and `to_pc` to the
  core, read both operands, and update the VM timestamp according to whether the
  branch is taken. The JAL/LUI and JALR adapters read only the operands mandated
  by the instruction and optionally skip the register write when `rd = x0`.
- **Multiplication family.** The multiplication, mulh, and div/rem adapters
  reuse the ALU adapter interface: two register reads, one write, and a
  program-counter increment of 4.

The sections below focus on the core-specific columns and constraints; adapter
side-effects (Program bus, memory access proofs, and timestamp updates) are
implied for every row.

## 1. Base ALU (add/addi/sub/xor/xori/or/ori/and/andi)

### 1.1 Columns

- a_0, a_1, a_2, a_3 — limbs of the value written to `rd`.
- b_0, b_1, b_2, b_3 — limbs of `rs1`.
- c_0, c_1, c_2, c_3 — limbs of `rs2` or the sign-extended immediate provided by
  the adapter.
- opcode_add_flag
- opcode_sub_flag
- opcode_xor_flag
- opcode_or_flag
- opcode_and_flag

### 1.2 Variables

- `is_valid = opcode_add_flag + opcode_sub_flag + opcode_xor_flag + opcode_or_flag + opcode_and_flag`.
- `carry_add[i] = (b_i + c_i + carry_add[i - 1] - a_i) / 2^RV32_CELL_BITS` with
  `carry_add[-1] = 0`.
- `carry_sub[i] = (a_i + c_i - b_i + carry_sub[i - 1]) / 2^RV32_CELL_BITS` with
  `carry_sub[-1] = 0`.
- `bitwise = opcode_xor_flag + opcode_or_flag + opcode_and_flag`.
- `expected_opcode = Σ flag_i * opcode_i + BaseAluOpcode::CLASS_OFFSET`.

### 1.3 Constraints

Each opcode flag is boolean and `is_valid` is also boolean, forcing exactly one
opcode to be active per row.

- `opcode_*_flag * (1 - opcode_*_flag)`
- `is_valid * (1 - is_valid)`

Add and sub enforce word-size carry chains. Whenever the corresponding flag is
1, every carry value must be boolean, which implies `a` equals
`(b ± c) mod 2^32` and forces the limbs to lie in `[0, 2^8)`.

- `opcode_add_flag * carry_add[i] * (1 - carry_add[i])`
- `opcode_sub_flag * carry_sub[i] * (1 - carry_sub[i])`

Bitwise operations share a single lookup. When `bitwise = 0`, the lookup
degenerates to `send_xor(a_i, a_i, 0)` and range-checks `a_i`. When one of
XOR/OR/AND is selected, the lookup enforces the appropriate truth table and
therefore the correctness of `a_i`.

- `BitwiseOperationLookupBus::send_xor(x_i, y_i, x_xor_y_i)` where `x_i` and
  `y_i` reduce to `b_i`/`c_i` only when `bitwise = 1`.

The adapter ensures the `c` limbs either come from `rs2` or from the injected
immediate, proves the Program-ROM read, and writes `a` back to `rd` (skipping
`x0`) while incrementing `pc` by `DEFAULT_PC_STEP`.

## 2. Shift (sll/slli/srl/srli/sra/srai)

### 2.1 Columns

- a_0…a_3, b_0…b_3, c_0…c_3 — limbs of the destination, `rs1`, and the shift
  operand or injected immediate.
- opcode_sll_flag, opcode_srl_flag, opcode_sra_flag.
- bit_multiplier_left, bit_multiplier_right — equal to `2^bit_shift` for left or
  right shifts.
- b_sign — sign bit replicated over the limb for SRA.
- bit_shift_marker[0:RV32_CELL_BITS) — one-hot encoding of the intra-limb shift.
- limb_shift_marker[0:RV32_REGISTER_NUM_LIMBS) — one-hot encoding of the limb
  offset.
- bit_shift_carry[0:RV32_REGISTER_NUM_LIMBS) — carry parts that cross limb
  boundaries.

### 2.2 Variables

- `is_valid = opcode_sll_flag + opcode_srl_flag + opcode_sra_flag`.
- `right_shift = opcode_srl_flag + opcode_sra_flag`.
- `bit_shift = Σ i * bit_shift_marker[i]`.
- `limb_shift = Σ i * limb_shift_marker[i]`.
- `shift_amount = limb_shift * RV32_CELL_BITS + bit_shift`.
- `num_bits = RV32_REGISTER_NUM_LIMBS * RV32_CELL_BITS`.

### 2.3 Constraints

Each opcode flag, every bit marker, and both multipliers are boolean or selected
via a boolean. The markers must sum to 1 whenever the row is valid, making
`bit_shift` and `limb_shift` unique.

- `Σ bit_shift_marker[i] = is_valid`
- `Σ limb_shift_marker[i] = is_valid`
- `bit_multiplier_left = opcode_sll_flag * 2^bit_shift`
- `bit_multiplier_right = right_shift * 2^bit_shift`

Left shifts zero the lower limbs and propagate the within-limb carry via
`bit_shift_carry`. Right shifts propagate carries backward and optionally fill
with `b_sign` when SRA is active. Each carry is range-checked to
`[0, 2^bit_shift)`.

- `a_j = 0` for `j < limb_shift` when SLL is selected.
- `a_j = b_{j - limb_shift} * bit_multiplier_left - 2^8 * bit_shift_carry` for
  `j ≥ limb_shift`.
- `a_j * bit_multiplier_right = right_shift * (b_{j + limb_shift} - bit_shift_carry_{j + limb_shift}) + carry_term`
- `RangeCheckerBus::send(bit_shift_carry_j, bit_shift)` when `is_valid = 1`.

The total shift amount stored in `c_0` must equal
`limb_shift * RV32_CELL_BITS + bit_shift (mod num_bits)`. The variable range
checker enforces that relationship and caps the amount at 31.

- `VariableRangeCheckerBus::range_check((c_0 - limb_shift * RV32_CELL_BITS - bit_shift) / num_bits, …)`

`b_sign` is boolean and is forced to 0 unless SRA is active. The top limb of `b`
is checked against `b_sign` via a bitwise lookup so that sign extension fills
the empty bits correctly.

The adapter handles immediate decoding for `slli`, `srli`, and `srai`, reads
both operands for the register forms, updates `pc + 4`, and writes the limbs of
`a` back to `rd`.

## 3. Less than (slt/slti/sltu/sltiu)

### 3.1 Columns

- b_0…b_3, c_0…c_3 — operands.
- cmp_result — boolean output bit that becomes `a_0`.
- opcode_slt_flag, opcode_sltu_flag.
- b_msb_f, c_msb_f — signed or unsigned views of the most significant limb.
- diff_marker[0:RV32_REGISTER_NUM_LIMBS) — one-hot marker of the first differing
  limb.
- diff_val — stores the signed difference at the differing limb.

### 3.2 Variables

- `is_valid = opcode_slt_flag + opcode_sltu_flag`.
- `marker_sum = Σ diff_marker[i]`.

### 3.3 Constraints

Both opcode flags and `cmp_result` are boolean. `marker_sum` is boolean whenever
`is_valid = 1`; it is 0 only if `b = c`.

- `opcode_*_flag * (1 - opcode_*_flag)`
- `cmp_result * (1 - cmp_result)`
- `marker_sum * (1 - marker_sum) = 0`

The top limbs are range-checked. When `opcode_slt_flag = 1`, both `b_msb_f` and
`c_msb_f` must lie in `[-128, 127)`; otherwise they lie in `[0, 256)`. The
bitwise lookup bus enforces both ranges simultaneously.

Iterating from the most significant limb downwards, the AIR ensures that
`diff_marker[i]` is 1 exactly at the first index where the operands differ and
that `diff_val` stores the signed difference that determines the comparison.
When no difference exists, `cmp_result` must be 0.

- `diff_marker[i] * (diff_marker[i] - 1) = 0`
- `Σ_{j ≥ i} diff_marker[j]` gates whether `(c_i - b_i)` must be zero.
- `diff_val = (c_i - b_i)` (signed) at the marked limb.

`diff_val` is range-checked to ensure it is non-zero whenever `marker_sum = 1`.
The output array passed back to the adapter has `a_0 = cmp_result` and zeros
elsewhere.

- `BitwiseOperationLookupBus::send_range(diff_val - 1, 0)` with multiplicity
  `marker_sum`.

The adapter reads `rs1`/`rs2` or injects the immediate, writes back the boolean
result, and increments `pc` by 4.

## 4. Branch equal (beq/bne)

### 4.1 Columns

- a_0…a_3, b_0…b_3 — branch operands.
- cmp_result — equals 1 when the branch condition is true.
- imm — signed branch offset.
- opcode_beq_flag, opcode_bne_flag.
- diff_inv_marker[0:RV32_REGISTER_NUM_LIMBS) — stores the inverse of `a_i - b_i`
  when the operands differ.

### 4.2 Variables

- `is_valid = opcode_beq_flag + opcode_bne_flag`.
- `cmp_eq = cmp_result * opcode_beq_flag + (1 - cmp_result) * opcode_bne_flag`.
- `sum = cmp_eq + Σ (a_i - b_i) * diff_inv_marker[i]`.

### 4.3 Constraints

Flags and `cmp_result` are boolean, and `sum` must equal 1 whenever the row is
valid.

- `opcode_*_flag * (1 - opcode_*_flag)`
- `cmp_result * (1 - cmp_result)`
- `sum = 1`

When `cmp_eq = 1`, all markers must be 0, forcing `a_i = b_i` for every limb.
When `cmp_eq = 0`, exactly one marker can be 1 and it must contain
`(a_i - b_i)^{-1}`, proving that the operands differ at that limb.

- `cmp_eq * (a_i - b_i) = 0`
- `diff_inv_marker[i] * (diff_inv_marker[i] * (a_i - b_i) - 1) = 0` whenever
  `cmp_eq = 0`.

The branch target is computed inside the core:

- `to_pc = from_pc + cmp_result * imm + (1 - cmp_result) * DEFAULT_PC_STEP`.

The adapter reads both registers, feeds `to_pc` to the execution bridge, and
updates the program counter accordingly.

## 5. Branch less than (blt/bge/bltu/bgeu)

### 5.1 Columns

- a_0…a_3, b_0…b_3 — operands.
- cmp_result — branch condition bit.
- imm — signed branch offset.
- opcode_blt_flag, opcode_bltu_flag, opcode_bge_flag, opcode_bgeu_flag.
- a_msb_f, b_msb_f — signed or unsigned views of the top limb.
- cmp_lt — equals `cmp_result` for `<` opcodes and `(1 - cmp_result)` for ≥
  opcodes.
- diff_marker[0:RV32_REGISTER_NUM_LIMBS), diff_val — same role as in §3.

### 5.2 Variables

- `is_valid = Σ opcode_*_flag`.
- `lt = opcode_blt_flag + opcode_bltu_flag`.
- `ge = opcode_bge_flag + opcode_bgeu_flag`.
- `signed = opcode_blt_flag + opcode_bge_flag`.

### 5.3 Constraints

Flags and `cmp_result` are boolean, and `cmp_lt` connects the boolean outputs of
the `<` and `≥` opcodes:

- `cmp_lt = cmp_result * lt + (1 - cmp_result) * ge`.

`a_msb_f`/`b_msb_f` are range-checked to signed or unsigned ranges depending on
`signed`. The diff-marker logic matches §3 but compares the limbs in the order
required for `<` comparisons. If no difference exists, `cmp_lt` must be 0.

- `BitwiseOperationLookupBus::send_range(a_msb_f + signed * 2^7, b_msb_f + signed * 2^7)`
- `diff_marker` and `diff_val` equations identical to §3.

As in §4, the target program counter is
`to_pc = from_pc + cmp_result * imm + (1 - cmp_result) * DEFAULT_PC_STEP`, and
the adapter enforces the memory and program-bus effects.

## 6. JAL and LUI (jal/lui)

### 6.1 Columns

- imm — signed jump offset (jal) or the 20-bit U-type immediate (lui), provided
  by the adapter.
- rd_data_0…rd_data_3 — limbs of the value written to `rd`.
- is_jal, is_lui — opcode selectors.

### 6.2 Variables

- `is_valid = is_jal + is_lui`.
- `last_limb_bits = PC_BITS - RV32_CELL_BITS * (RV32_REGISTER_NUM_LIMBS - 1)`.
- `additional_bits = Σ_{k=last_limb_bits}^{RV32_CELL_BITS-1} 2^k`.
- `intermed_val = Σ_{i=1}^{3} rd_data_i * 2^{i * RV32_CELL_BITS}`.

### 6.3 Constraints

`is_jal` and `is_lui` are boolean and must not be simultaneously 1. `rd_data_0`
is forced to 0 on LUI rows.

- `is_jal * is_lui = 0`
- `is_lui * rd_data_0 = 0`

For JAL, the intermediate value plus `rd_data_0` must equal `from_pc + 4`, and
the most significant limb is range-checked so that only `PC_BITS`-many bits are
used.

- `intermed_val + rd_data_0 = from_pc + DEFAULT_PC_STEP`
- `BitwiseOperationLookupBus::send_xor(rd_data_3, additional_bits, rd_data_3 + additional_bits)`
  with multiplicity `is_jal`

For LUI, `intermed_val` equals the decomposed immediate shifted left by 12 bits
(the adapter pre-divides `imm` so multiplying by `2^{12 - RV32_CELL_BITS}`
reconstructs the proper limb alignment). The lower byte remains zero, so
`compose(rd) = imm << 12`.

The branch target equals `from_pc + imm` for JAL and `from_pc + 4` for LUI:

- `to_pc = from_pc + is_lui * DEFAULT_PC_STEP + is_jal * imm`.

The adapter does not read any register, conditionally writes `rd`, and sets the
next PC to `to_pc`.

## 7. JALR (jalr)

### 7.1 Columns

- imm — low 16 bits of the immediate (the adapter provides `imm_sign` for
  extending).
- rs1_data_0…rs1_data_3 — source limbs.
- rd_data_0…rd_data_2 — the three most significant limbs of `pc + 4` (the least
  significant limb is reconstructed from `from_pc`).
- is_valid
- to_pc_least_sig_bit — bit 0 of `rs1 + imm`, forced to 0 by the spec.
- to_pc_limbs_0, to_pc_limbs_1 — 16-bit limbs of `2 * to_pc`.
- imm_sign — boolean sign bit describing whether the original immediate was
  negative.

### 7.2 Variables

- `least_sig_limb = from_pc + DEFAULT_PC_STEP - Σ_{i=1}^{3} rd_data_{i-1} * 2^{i * RV32_CELL_BITS}`.
- `rd = compose([least_sig_limb, rd_data_0, rd_data_1, rd_data_2])`.
- `rs1_lo = rs1_data_0 + rs1_data_1 * 2^{RV32_CELL_BITS}`.
- `rs1_hi = rs1_data_2 + rs1_data_3 * 2^{RV32_CELL_BITS}`.
- `imm_extend = imm_sign * (2^{16} - 1)`.

### 7.3 Constraints

`is_valid`, `imm_sign`, and `to_pc_least_sig_bit` are boolean. The reconstructed
`rd` must equal `from_pc + 4`, and each limb is range-checked via the bitwise
lookup bus and variable range checker.

- `BitwiseOperationLookupBus::send_range(rd_data_0, rd_data_1)`
- `RangeCheckerBus::range_check(rd_data_2, RV32_CELL_BITS)`
- `RangeCheckerBus::range_check(rd_data_3, PC_BITS - 3 * RV32_CELL_BITS)`

`to_pc_least_sig_bit` enforces the RISC-V requirement that bit 0 is zero, so the
main addition is performed on `2 * to_pc`. Two chained carry equations enforce
`(rs1 + imm) & !1`:

- `carry = (rs1_lo + imm - 2 * to_pc_limbs_0 - to_pc_least_sig_bit) / 2^{16}`
- `carry` boolean
- `next_carry = (rs1_hi + imm_extend + carry - to_pc_limbs_1) / 2^{16}`
- `next_carry` boolean

The range checker constrains `to_pc_limbs_0 < 2^{15}` and
`to_pc_limbs_1 < 2^{PC_BITS - 16}`, implying `to_pc < 2^{PC_BITS}`. The
`MinimalInstruction` passed to the adapter wraps `imm`, `imm_sign`, and the
computed `to_pc`.

## 8. AUIPC (auipc)

### 8.1 Columns

- is_valid
- imm_limbs_0…imm_limbs_2 — the upper three limbs of the U-type immediate (the
  least significant byte is known to be zero).
- pc_limbs_0…pc_limbs_1 — the middle limbs of `pc`.
- rd_data_0…rd_data_3 — the result limbs.

### 8.2 Variables

- `pc_intermediate = rd_data_0 + pc_limbs_0 * 2^{RV32_CELL_BITS} + pc_limbs_1 * 2^{2 * RV32_CELL_BITS}`.
- `pc_msl = (from_pc - pc_intermediate) / 2^{3 * RV32_CELL_BITS}` — the most
  significant limb.
- `pc_limbs = [rd_data_0, pc_limbs_0, pc_limbs_1, pc_msl]`.
- `carry[i] = (pc_limbs_i + imm_limbs_{i - 1} + carry_{i - 1} - rd_data_i) / 2^{RV32_CELL_BITS}`
  for `i ≥ 1`.

### 8.3 Constraints

`is_valid` is boolean and every `carry[i]` must be boolean when `is_valid = 1`,
proving `rd = pc + imm`.

Each pair of `rd` limbs is range-checked via the bitwise lookup bus. The imm and
pc limbs are also range-checked in pairs; the most significant pc limb is scaled
so that it fits inside its reduced bit width.

- `BitwiseOperationLookupBus::send_range(rd_data_{2i}, rd_data_{2i + 1})`
- `BitwiseOperationLookupBus::send_range(imm_limb_i, imm_limb_{i + 1})`
- `BitwiseOperationLookupBus::send_range(pc_limb_i, pc_limb_{i + 1})`

Finally, the immediate that is stored in the emitted instruction equals
`Σ imm_limbs_i * 2^{i * RV32_CELL_BITS}`, so the adapter can feed it back to the
program bus. No register reads are needed; the adapter writes `rd` and
increments `pc` by `DEFAULT_PC_STEP`.

## 9. Load sign extend (lb/lh)

### 9.1 Columns

- opcode_loadb_flag0 — selector when the addressed byte lies in the low
  half-word.
- opcode_loadb_flag1 — selector when the addressed byte lies in the high
  half-word.
- opcode_loadh_flag — selector for half-word loads (shift encoded separately).
- shift_most_sig_bit — the second-lowest address bit (0 for offsets {0, 1}, 1
  for {2, 3}).
- data_most_sig_bit — the sign bit that will be extended.
- shifted_read_data_0…shifted_read_data_3 — the four bytes returned by the
  adapter after applying the pointer-dependent rotation.
- prev_data_0…prev_data_3 — previous register contents (forwarded to keep the
  adapter interface uniform).

### 9.2 Variables

- `flags = opcode_loadb_flag0 + opcode_loadb_flag1 + opcode_loadh_flag`.
- `is_valid = flags`.
- `load_shift_amount = shift_most_sig_bit * 2 + opcode_loadb_flag1`.
- `most_sig_limb = opcode_loadb_flag0 * shifted_read_data_0 + opcode_loadb_flag1 * shifted_read_data_1 + opcode_loadh_flag * shifted_read_data_{RV32_REGISTER_NUM_LIMBS / 2 - 1}`.
- `limb_mask = data_most_sig_bit * (2^{RV32_CELL_BITS} - 1)`.
- `write_data =` first limb equals the selected byte, the next half-word equals
  either the shifted half-word or `limb_mask`, and the upper half equals
  `limb_mask`.
- `read_data[i] = shift_most_sig_bit ? shifted_read_data[(i + RV32_REGISTER_NUM_LIMBS - 2) mod RV32_REGISTER_NUM_LIMBS] : shifted_read_data[i]`.

### 9.3 Constraints

Each flag plus `shift_most_sig_bit` and `data_most_sig_bit` are boolean, and
`is_valid` is boolean. The range checker enforces that `most_sig_limb` and
`data_most_sig_bit` agree on the sign bit.

- `data_most_sig_bit * (1 - data_most_sig_bit) = 0`
- `VariableRangeCheckerBus::range_check(most_sig_limb - data_most_sig_bit * 2^{RV32_CELL_BITS - 1}, RV32_CELL_BITS - 1)`

`write_data` is constructed limb-by-limb inside the core, so the adapter only
needs to copy it into the register file. The `LoadStoreInstruction` emitted by
the core sets `is_load = is_valid`, `load_shift_amount` as above, and
`store_shift_amount = 0`, and the adapter uses these markers to prove the
correct word-lane selection.

## 10. Load/store (lw/lbu/lhu/sw/sh/sb)

### 10.1 Columns

- flags[0:4) — encodes the 13 load/store/shift combinations described by
  `InstructionOpcode`.
- is_valid — indicates whether the row is populated.
- is_load — 1 for load opcodes, 0 for stores.
- read_data_0…read_data_3 — word read from memory.
- prev_data_0…prev_data_3 — the value read from `rd` (used to merge bytes for
  stores).
- write_data_0…write_data_3 — result limbs written back to the register file or
  to memory.

### 10.2 Variables

- `sum = Σ flags[i]`.
- `opcode_flags` — the 13 mutually exclusive booleans derived from quadratic
  polynomials in `flags`, matching each case listed in the source (`LoadW0`,
  `LoadHu0`, …, `StoreB3`).
- `opcode_when(S) = Σ_{case ∈ S} opcode_flag_case`.
- `load_shift_amount = opcode_when({LoadBu1}) + 2 * opcode_when({LoadHu2, LoadBu2}) + 3 * opcode_when({LoadBu3})`.
- `store_shift_amount = opcode_when({StoreB1}) + 2 * opcode_when({StoreH2, StoreB2}) + 3 * opcode_when({StoreB3})`.

### 10.3 Constraints

Each `flags[i]` is constrained to `{0, 1, 2}` via
`flags[i] * (flags[i] - 1) * (flags[i] - 2) = 0`, and `sum` is also forced into
`{0, 1, 2}`. When `sum = 0` the row is unused and `is_valid = 0`; otherwise
`is_valid = 1`. `is_load` equals
`opcode_when({LoadW0, LoadHu0, LoadHu2, LoadBu0, LoadBu1, LoadBu2, LoadBu3})`.

`write_data` is recomputed exactly as in `run_write_data` inside the core. For
load opcodes the corresponding limbs of `read_data` are selected; for stores the
byte lanes are merged with `prev_data` so only the addressed bytes change.

- `write_data_0 = opcode_when({LoadW0, LoadHu0, LoadBu0}) * read_data_0 + opcode_when({LoadBu1}) * read_data_1 + …`

The expected opcode is a linear combination of the selectors and the
`Rv32LoadStoreOpcode` discriminants. The LoadStoreInstruction handed to the
adapter contains `is_valid`, `is_load`, and the precomputed shift amounts; the
adapter enforces address-space and pointer constraints and either writes
`write_data` to the register file (loads) or uses it as the data to be written
to memory (stores).

## 11. Multiplication (mul)

### 11.1 Columns

- a_0…a_3 — limbs of the low word of the product.
- b_0…b_3, c_0…c_3 — operands.
- is_valid

### 11.2 Variables

- `carry[i] = (Σ_{k=0}^{i} b_k * c_{i - k} + carry[i - 1] - a_i) / 2^{RV32_CELL_BITS}`
  with `carry[-1] = 0`.

### 11.3 Constraints

`is_valid` is boolean. Each `carry[i]` is forced to lie in `[0, 2^8)` via the
`RangeTupleCheckerBus`, proving that `a` equals `(b * c) mod 2^32`.

- `RangeTupleCheckerBus::send([a_i, carry_i])`

The adapter supplies the operands, writes `a`, and increments the program
counter by 4.

## 12. MULH family (mulh/mulhu/mulhsu)

### 12.1 Columns

- a_0…a_3 — limbs of the high word of the product.
- b_0…b_3, c_0…c_3 — operands.
- a_mul_0…a_mul_3 — the low word of `b * c`.
- b_ext, c_ext — sign-extension masks (0 or `2^{RV32_CELL_BITS} - 1`).
- opcode_mulh_flag, opcode_mulhsu_flag, opcode_mulhu_flag.

### 12.2 Variables

- `a_mul_carry[i] = (Σ_{k=0}^{i} b_k * c_{i - k} + a_mul_carry[i - 1] - a_mul_i) / 2^{RV32_CELL_BITS}`.
- `carry[i] = (carry_input(i) + Σ_{k=i+1}^{3} b_k * c_{i + 3 - k} + Σ_{k=0}^{i} (b_k * c_ext + c_k * b_ext) - a_i) / 2^{RV32_CELL_BITS}`,
  where `carry_input(0) = a_mul_carry[3]` and
  `carry_input(i > 0) = carry[i - 1]`.
- `b_sign = b_ext / (2^{RV32_CELL_BITS} - 1)`,
  `c_sign = c_ext / (2^{RV32_CELL_BITS} - 1)`.

### 12.3 Constraints

All opcode flags are boolean and sum to `is_valid`. The low-word carries enforce
that `a_mul` equals `(b * c) mod 2^32`, while the second set of carries enforces
that `a` equals the high word plus the contributions from the sign extensions.
Every `(a_mul_i, a_mul_carry_i)` and `(a_i, carry_i)` pair passes through the
range-tuple checker.

`b_ext` and `c_ext` encode whether each operand is interpreted as signed.
`b_sign` and `c_sign` are boolean. `opcode_mulhu_flag` forces both `b_sign` and
`c_sign` to 0; `opcode_mulhsu_flag` forces `c_sign = 0` but leaves `b_sign`
free; `opcode_mulh_flag` allows both signs to be set. The bitwise lookup bus
checks that the selected sign matches the MSB of the operand’s top limb.

- `BitwiseOperationLookupBus::send_range(2 * (b_3 - b_sign * 2^{RV32_CELL_BITS - 1}), 2 * (c_3 - c_sign * 2^{RV32_CELL_BITS - 1}))`

## 13. Division and remainder (div/divu/rem/remu)

### 13.1 Columns

- b_0…b_3, c_0…c_3 — dividend and divisor.
- q_0…q_3, r_0…r_3 — quotient and remainder limbs.
- zero_divisor — boolean flag indicating `c = 0`.
- r_zero — boolean flag indicating `r = 0`.
- b_sign, c_sign, q_sign, sign_xor — boolean sign markers.
- c_sum_inv, r_sum_inv — inverses used to prove that `c` and `r` are non-zero
  when their flags are unset.
- r_prime_0…r_prime_3 — magnitude of the remainder.
- r_inv_0…r_inv_3 — inverses that constrain `r_prime_i ≠ 2^{RV32_CELL_BITS}`
  when `sign_xor = 1`.
- lt_marker[0:RV32_REGISTER_NUM_LIMBS), lt_diff — unsigned-compare helper for
  `|r| < |c|`.
- opcode_div_flag, opcode_divu_flag, opcode_rem_flag, opcode_remu_flag.

### 13.2 Variables

- `is_valid = Σ opcode_*_flag`.
- `b_ext = b_sign * (2^{RV32_CELL_BITS} - 1)`,
  `c_ext = c_sign * (2^{RV32_CELL_BITS} - 1)`.
- `q_ext = q_sign * (2^{RV32_CELL_BITS} - 1)`.
- `special_case = zero_divisor + r_zero`.

### 13.3 Constraints

Opcode flags and all sign flags are boolean; exactly one opcode is active per
row. The first carry chain enforces `b = c * q + r (mod 2^{32})` by checking
`(q_i, carry_i)` tuples with the range-tuple checker.

The second carry chain enforces that the high limbs of `c * q + r` equal
`b_ext`. When `zero_divisor = 1`, every limb of `c` must be zero and each `q_i`
is forced to `2^{RV32_CELL_BITS} - 1`. Conversely, when `zero_divisor = 0` the
inverse `c_sum_inv` proves that not all `c_i` are zero. An analogous invariant
holds for `r_zero` and `r_sum_inv`.

`b_sign` and `c_sign` are compared with the top limbs via the bitwise lookup
bus. For unsigned instructions (`divu`, `remu`) both signs must be zero.
`q_sign` equals `b_sign XOR c_sign` whenever the quotient is non-zero and
`zero_divisor = 0`.

`r_prime` equals `r` when the signs match and `-r` otherwise. When signs differ,
a carry chain forces `r + r_prime` to be either 0 or `2^{RV32_CELL_BITS}`, and
`r_inv_i` proves that `r_prime_i ≠ 2^{RV32_CELL_BITS}`. The markers `lt_marker`
and `lt_diff` run the same MSB-to-LSB logic as §5 to prove that `r_prime` is
strictly less than `c` (unsigned) whenever no special case is active. The
bitwise lookup bus range-checks `lt_diff - 1` so that `lt_diff ≠ 0`.

Only one special-case flag may be set: `special_case` is boolean, and when it
equals 1 the `lt_marker` constraints are skipped while the adapters’
post-processing logic enforces the RISC-V mandated outputs (`-1` for signed
division by zero, `2^32 - 1` for unsigned).

The output written back to the register file equals `q` for DIV/DIVU and `r` for
REM/REMU, implemented as
`a_i = opcode_is_div * q_i + (1 - opcode_is_div) * r_i`. The adapter reads both
operands, writes the result, and advances `pc` by 4.
