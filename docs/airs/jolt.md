# AIRS

The constraints below describe the per-cycle AIR used by Jolt's CPU when proving
RV32IM execution traces. Each section follows the same format as the reference
`../stark-v/airs.md` document: we first list the columns that appear in the row,
then the derived variables, and finally the constraints that must vanish. The
column names all refer to the virtual polynomials introduced in
`jolt-core/src/zkvm/r1cs/inputs.rs`:

- `PC`, `NextPC`, `UnexpandedPC`, `NextUnexpandedPC` – program counter columns.
- `RdIdx`, `Rs1Idx`, `Rs2Idx` together with `rd_prev`, `rd_next`, `rs1_val`,
  `rs2_val` – register indices and the values captured by the register Twist
  instance.
- `Imm` – the RISC-V immediate decoded during preprocessing.
- `LeftInstructionInput`, `RightInstructionInput` – operands fed to the
  instruction lookup gadget.
- `LeftLookupOperand`, `RightLookupOperand`, `LookupOutput` – the lookup tuple
  sent to Shout.
- `RamAddress`, `RamReadValue`, `RamWriteValue` – RAM access metadata.
- `OpFlags(flag)` – Boolean circuit flags; `InstrFlags(flag)` – Boolean selector
  flags derived from the opcode (e.g. `LeftOperandIsRs1Value`).

We freely use helper gadgets that come from other components:

- `RegsRead(rs, val)` ensures that the register file exposes `val` at index
  `rs`.
- `RegsWrite(rd, prev, next)` enforces the write to `rd` and is enabled only
  when `rd ≠ 0`.
- `RamLoad(addr, val)` / `RamStore(addr, pre, post)` are enforced by the Twist
  RAM instance whenever the load/store flags are set.
- `Bytecode(pc, rd, rs1, rs2, imm, flags)` ties the decoded operands and flags
  to the program bytecode via Shout.

Whenever we refer to `RangeCheckTable`, `AndTable`, `OrTable`, `XorTable`,
`SignedLessThanTable`, `UnsignedLessThanTable`, or `EqualTable`, we use the
lookup tables defined under `jolt-core/src/zkvm/lookup_table`.

## 1. lui

### 1.1 Columns

- `PC`, `UnexpandedPC`, `NextUnexpandedPC`
- `RdIdx`, `rd_prev`, `rd_next`
- `Imm`
- `LeftInstructionInput`, `RightInstructionInput`
- `LeftLookupOperand`, `RightLookupOperand`, `LookupOutput`
- `OpFlags(AddOperands)`, `OpFlags(WriteLookupOutputToRD)`
- `InstrFlags(RightOperandIsImm)`, `InstrFlags(IsRdNotZero)`

### 1.2 Variables

- `imm = Imm`
- `result = imm mod 2^XLEN`

### 1.3 Constraints

enabler (the decoded opcode) updates the bytecode tuple

- `Bytecode(PC, RdIdx, 0, 0, imm, flags)`

instruction operands and lookup wiring

- `LeftInstructionInput = 0`
- `RightInstructionInput = imm`
- `LeftLookupOperand = 0` (because `OpFlags(AddOperands) = 1`)
- `RightLookupOperand = imm`
- `LookupOutput = RangeCheckTable(result)`

register update (skips `x0`)

- `RegsRead(RdIdx, rd_prev)`
- `RegsWrite(RdIdx, rd_prev, rd_next)` with `rd_next = LookupOutput`

program counter

- `NextUnexpandedPC = UnexpandedPC + (is_compressed ? 2 : 4)`

## 2. auipc

### 2.1 Columns

- `PC`, `UnexpandedPC`, `NextUnexpandedPC`
- `RdIdx`, `rd_prev`, `rd_next`
- `Imm`
- `LeftInstructionInput`, `RightInstructionInput`
- `LeftLookupOperand`, `RightLookupOperand`, `LookupOutput`
- `OpFlags(AddOperands)`, `OpFlags(WriteLookupOutputToRD)`
- `InstrFlags(LeftOperandIsPC)`, `InstrFlags(RightOperandIsImm)`,
  `InstrFlags(IsRdNotZero)`

### 2.2 Variables

- `pc = UnexpandedPC`
- `imm = Imm`
- `result = (pc + imm) mod 2^XLEN`

### 2.3 Constraints

- `LeftInstructionInput = pc`
- `RightInstructionInput = imm`
- `LeftLookupOperand = 0`
- `RightLookupOperand = pc + imm`
- `LookupOutput = RangeCheckTable(result)`
- `RegsRead(RdIdx, rd_prev)` and `RegsWrite(RdIdx, rd_prev, rd_next)` with
  `rd_next = LookupOutput`
- `NextUnexpandedPC = UnexpandedPC + (is_compressed ? 2 : 4)`

## 3. jal

### 3.1 Columns

- `PC`, `NextPC`, `UnexpandedPC`, `NextUnexpandedPC`
- `RdIdx`, `rd_prev`, `rd_next`
- `Imm`
- `LeftInstructionInput`, `RightInstructionInput`
- `LeftLookupOperand`, `RightLookupOperand`, `LookupOutput`
- `OpFlags(Jump)`, `OpFlags(WriteLookupOutputToRD)`, `OpFlags(AddOperands)`
- `InstrFlags(LeftOperandIsPC)`, `InstrFlags(RightOperandIsImm)`,
  `InstrFlags(IsRdNotZero)`

### 3.2 Variables

- `pc = UnexpandedPC`
- `imm = Imm`
- `target = pc + imm`
- `ret = pc + 4 - 2 * OpFlags(IsCompressed)`

### 3.3 Constraints

- `LeftInstructionInput = pc`
- `RightInstructionInput = imm`
- `RightLookupOperand = pc + imm`
- `LookupOutput = target`
- `ShouldJump = 1`, so `NextUnexpandedPC = LookupOutput`
- `RegsWrite(RdIdx, rd_prev, rd_next)` with `rd_next = ret`
- `NextPC = PC + 1` when the instruction belongs to a virtual sequence,
  otherwise `NextPC = PC + 1` holds trivially because `JAL` increments the
  expanded trace by one step.

## 4. jalr

### 4.1 Columns

- `PC`, `NextPC`, `UnexpandedPC`, `NextUnexpandedPC`
- `RdIdx`, `rd_prev`, `rd_next`
- `Rs1Idx`, `rs1_val`
- `Imm`
- `LeftInstructionInput`, `RightInstructionInput`
- `LeftLookupOperand`, `RightLookupOperand`, `LookupOutput`
- `OpFlags(Jump)`, `OpFlags(AddOperands)`, `OpFlags(WriteLookupOutputToRD)`
- `InstrFlags(LeftOperandIsRs1Value)`, `InstrFlags(RightOperandIsImm)`,
  `InstrFlags(IsRdNotZero)`

### 4.2 Variables

- `base = rs1_val`
- `imm = Imm`
- `raw_target = base + imm`
- `target = (raw_target & !1)`
- `ret = UnexpandedPC + 4 - 2 * OpFlags(IsCompressed)`

### 4.3 Constraints

- `RegsRead(Rs1Idx, rs1_val)`
- `LeftInstructionInput = rs1_val`
- `RightInstructionInput = imm`
- `RightLookupOperand = raw_target`
- `LookupOutput = raw_target & !1`
- `ShouldJump = 1`, so `NextUnexpandedPC = LookupOutput`
- `RegsWrite(RdIdx, rd_prev, rd_next)` with `rd_next = ret`

## 5. beq

### 5.1 Columns

- `PC`, `UnexpandedPC`, `NextUnexpandedPC`
- `Rs1Idx`, `rs1_val`, `Rs2Idx`, `rs2_val`
- `Imm`
- `LeftInstructionInput`, `RightInstructionInput`
- `LeftLookupOperand`, `RightLookupOperand`, `LookupOutput`
- `OpFlags(Branch helpers)` only through `InstrFlags(Branch)`
- `InstrFlags(LeftOperandIsRs1Value)`, `InstrFlags(RightOperandIsRs2Value)`,
  `InstrFlags(Branch)`

### 5.2 Variables

- `lhs = rs1_val`
- `rhs = rs2_val`
- `imm = Imm`
- `cond = (lhs == rhs)`

### 5.3 Constraints

- `RegsRead(Rs1Idx, lhs)` and `RegsRead(Rs2Idx, rhs)`
- `LeftInstructionInput = lhs`, `RightInstructionInput = rhs`
- `LookupOutput = EqualTable(lhs, rhs)`
- `ShouldBranch = LookupOutput`
- `NextUnexpandedPC = if cond { UnexpandedPC + imm } else { UnexpandedPC + (is_compressed ? 2 : 4) }`

## 6. bne

### 6.1 Columns

Same as `beq`.

### 6.2 Variables

- `cond = (rs1_val != rs2_val)`

### 6.3 Constraints

- `LookupOutput = NotEqualTable(rs1_val, rs2_val)`
- `ShouldBranch = LookupOutput`
- `NextUnexpandedPC` updates as in `beq`.

## 7. blt

### 7.1 Columns

Same register, immediate, and lookup columns as `beq`.

### 7.2 Variables

- `cond = (rs1_val < rs2_val)` interpreted as signed integers.

### 7.3 Constraints

- `LookupOutput = SignedLessThanTable(rs1_val, rs2_val)`
- `ShouldBranch = LookupOutput`
- `NextUnexpandedPC` updates as for other branches.

## 8. bge

### 8.1 Columns

Same as `blt`.

### 8.2 Variables

- `cond = (rs1_val ≥ rs2_val)` signed.

### 8.3 Constraints

- `LookupOutput = 1 - SignedLessThanTable(rs1_val, rs2_val)`
- Remaining constraints identical to `blt`.

## 9. bltu

### 9.1 Columns

Same as `blt`.

### 9.2 Variables

- `cond = (rs1_val < rs2_val)` interpreted as unsigned.

### 9.3 Constraints

- `LookupOutput = UnsignedLessThanTable(rs1_val, rs2_val)`
- `ShouldBranch = LookupOutput`
- `NextUnexpandedPC` updates as usual.

## 10. bgeu

### 10.1 Columns

Same as `bltu`.

### 10.2 Variables

- `cond = (rs1_val ≥ rs2_val)` unsigned.

### 10.3 Constraints

- `LookupOutput = 1 - UnsignedLessThanTable(rs1_val, rs2_val)`
- Remaining constraints identical to `bltu`.

## 11. and

### 11.1 Columns

- `Rs1Idx`, `rs1_val`, `Rs2Idx`, `rs2_val`
- `RdIdx`, `rd_prev`, `rd_next`
- `LeftInstructionInput`, `RightInstructionInput`
- `LeftLookupOperand`, `RightLookupOperand`, `LookupOutput`
- `OpFlags(WriteLookupOutputToRD)`
- `InstrFlags(LeftOperandIsRs1Value)`, `InstrFlags(RightOperandIsRs2Value)`,
  `InstrFlags(IsRdNotZero)`

### 11.2 Variables

- `lhs = rs1_val`
- `rhs = rs2_val`
- `result = lhs & rhs`

### 11.3 Constraints

- `LeftInstructionInput = lhs`, `RightInstructionInput = rhs`
- `LeftLookupOperand = lhs`, `RightLookupOperand = rhs`
- `LookupOutput = AndTable(lhs, rhs)`
- `RegsWrite(RdIdx, rd_prev, rd_next)` with `rd_next = LookupOutput`

## 12. or

Same structure as `and`, with `LookupOutput = OrTable(lhs, rhs)`.

## 13. xor

Same structure as `and`, with `LookupOutput = XorTable(lhs, rhs)`.

## 14. andi

### 14.1 Columns

- `Rs1Idx`, `rs1_val`
- `Imm`
- `RdIdx`, `rd_prev`, `rd_next`
- Lookup columns as before
- `InstrFlags(LeftOperandIsRs1Value)`, `InstrFlags(RightOperandIsImm)`

### 14.2 Variables

- `result = rs1_val & Imm`

### 14.3 Constraints

- `LeftInstructionInput = rs1_val`
- `RightInstructionInput = Imm`
- `LookupOutput = AndTable(rs1_val, Imm)`
- Register write as before.

## 15. ori

Identical to `andi`, but `LookupOutput = OrTable(rs1_val, Imm)`.

## 16. xori

Identical to `andi`, but `LookupOutput = XorTable(rs1_val, Imm)`.

## 17. andn

Same columns as `and`.

- Variables: `result = rs1_val & !rs2_val`.
- Constraints use the dedicated `AndTable` with negated operand implemented in
  the tracer, so `LookupOutput = AndTable(rs1_val, !rs2_val)`.

## 18. add

### 18.1 Columns

Same as `and`, except the lookup table is `RangeCheckTable` and
`OpFlags(AddOperands) = OpFlags(WriteLookupOutputToRD) = 1`.

### 18.2 Variables

- `result = (rs1_val + rs2_val) mod 2^XLEN`

### 18.3 Constraints

- `LeftInstructionInput = rs1_val`, `RightInstructionInput = rs2_val`
- `LeftLookupOperand = 0`
- `RightLookupOperand = rs1_val + rs2_val`
- `LookupOutput = RangeCheckTable(result)`
- Register write as before.

## 19. addi

Same as `add`, with `RightInstructionInput = Imm` and `result = rs1_val + Imm`.

## 20. sub

### 20.1 Columns

Same register and lookup columns as `add`.

### 20.2 Variables

- `result = (rs1_val - rs2_val) mod 2^XLEN`

### 20.3 Constraints

- `OpFlags(SubtractOperands) = 1`
- `RightLookupOperand = rs1_val - rs2_val + 2^XLEN`
- `LookupOutput = RangeCheckTable(result)`
- Register write as before.

## 21. slt

### 21.1 Columns

- `Rs1Idx`, `rs1_val`, `Rs2Idx`, `rs2_val`
- `RdIdx`, `rd_prev`, `rd_next`
- Lookup columns
- `InstrFlags(LeftOperandIsRs1Value)`, `InstrFlags(RightOperandIsRs2Value)`,
  `InstrFlags(IsRdNotZero)`

### 21.2 Variables

- `result = 1` iff `rs1_val < rs2_val` (signed), else `0`.

### 21.3 Constraints

- `LookupOutput = SignedLessThanTable(rs1_val, rs2_val)`
- `RegsWrite` stores that bit in `rd`.

## 22. sltu

Identical to `slt`, but uses `UnsignedLessThanTable`.

## 23. slti

Same as `slt`, except `RightInstructionInput = Imm`.

## 24. sltiu

Same as `sltu`, except `RightInstructionInput = Imm`.

## 25. mul

### 25.1 Columns

Same register/lookup columns as `add`, but with `OpFlags(MultiplyOperands) = 1`.

### 25.2 Variables

- `result = (rs1_val × rs2_val) mod 2^XLEN`

### 25.3 Constraints

- `RightLookupOperand = rs1_val × rs2_val`
- `LookupOutput = RangeCheckTable(result)`
- Register write as usual.

## 26. mulhu

### 26.1 Columns

Same as `mul`.

### 26.2 Variables

- `result = upper_half(rs1_val × rs2_val)` interpreted as unsigned values.

### 26.3 Constraints

- `LookupOutput = RangeCheckTable(result)` but interpreted with unsigned
  widening semantics by the tracer’s `to_lookup_output` implementation.

## 27. ld

### 27.1 Columns

- `Rs1Idx`, `rs1_val`
- `RdIdx`, `rd_prev`, `rd_next`
- `Imm`
- `RamAddress`, `RamReadValue`, `RamWriteValue`
- `OpFlags(Load)`
- `InstrFlags(IsRdNotZero)`

### 27.2 Variables

- `addr = rs1_val + Imm`
- `value = RamReadValue`

### 27.3 Constraints

- `RamAddress = addr`
- `RamReadValue = RamWriteValue`
- `RegsWrite(RdIdx, rd_prev, rd_next)` with `rd_next = value`
- `RamLoad(addr, value)` enforced by the RAM Twist
- `NextUnexpandedPC = UnexpandedPC + (is_compressed ? 2 : 4)`

## 28. sd

### 28.1 Columns

- `Rs1Idx`, `rs1_val`
- `Rs2Idx`, `rs2_val`
- `Imm`
- `RamAddress`, `RamReadValue`, `RamWriteValue`
- `OpFlags(Store)`

### 28.2 Variables

- `addr = rs1_val + Imm`
- `value = rs2_val`

### 28.3 Constraints

- `RamAddress = addr`
- `RamWriteValue = value`
- `RamStore(addr, RamReadValue, value)`
- No register write occurs.

## 29. ecall

### 29.1 Columns

- `PC`, `UnexpandedPC`, `NextUnexpandedPC`
- `InstrFlags(IsRdNotZero)` (only to keep uniform format)

### 29.2 Variables

None; the instruction performs no lookup.

### 29.3 Constraints

- No register or memory changes occur (`RegsWrite` disabled).
- `NextUnexpandedPC = UnexpandedPC + (is_compressed ? 2 : 4)`.

## 30. fence

Identical to `ecall` from the AIR point of view.

## 31. virtual_pow2

### 31.1 Columns

- `Rs1Idx`, `rs1_val`
- `RdIdx`, `rd_prev`, `rd_next`
- Lookup columns
- `OpFlags(AddOperands, WriteLookupOutputToRD)`
- `InstrFlags(LeftOperandIsRs1Value)`

### 31.2 Variables

- `shift = rs1_val mod XLEN`
- `result = 1 << shift`

### 31.3 Constraints

- `LeftLookupOperand = 0`
- `RightLookupOperand = rs1_val`
- `LookupOutput = Pow2Table(shift)`
- `rd_next = LookupOutput`

## 32. virtual_pow2i

Identical to `virtual_pow2`, except the shift amount comes from `Imm`.

## 33. virtual_muli

Behaves like `mul`, but the right operand is `Imm`.

## 34. virtual_shift_right_bitmask

### 34.1 Columns

- `Rs1Idx`, `rs1_val`, `RdIdx`, `rd_prev`, `rd_next`
- Lookup tuple with `ShiftRightBitmaskTable`
- `OpFlags(AddOperands, WriteLookupOutputToRD)`

### 34.2 Variables

- `shift = rs1_val mod XLEN`
- `result = ((2^XLEN - 1) << shift) & (2^XLEN - 1)`

### 34.3 Constraints

- `LookupOutput = ShiftRightBitmaskTable(rs1_val)`
- `rd_next = LookupOutput`

## 35. virtual_shift_right_bitmaski

Same behavior as `virtual_shift_right_bitmask`, but the shift amount comes from
`Imm`.

## 36. virtual_srl

### 36.1 Columns

- `Rs1Idx`, `rs1_val`, `Rs2Idx`, `rs2_mask`
- `RdIdx`, `rd_prev`, `rd_next`
- Lookup tuple with `VirtualSRLTable`

### 36.2 Variables

- `mask` is produced by the preceding `virtual_shift_right_bitmask` instruction.
- `result = (rs1_val & mask) >> (XLEN - popcount(mask))`

### 36.3 Constraints

- `LookupOutput = VirtualSRLTable(rs1_val, rs2_mask)`
- `rd_next = LookupOutput`

## 37. virtual_srli

Same as `virtual_srl`, but the mask is derived from an immediate and only the
right operand column is used.

## 38. virtual_sra / virtual_srai

Identical to the SRL variants, but the lookup table sign-extends the shifted
value to maintain arithmetic shift semantics.

## 39. virtual_advice

### 39.1 Columns

- `RdIdx`, `rd_prev`, `rd_next`
- Lookup tuple with `RangeCheckTable`
- `OpFlags(Advice)`, `OpFlags(WriteLookupOutputToRD)`

### 39.2 Variables

- `advice` is provided externally during tracing.

### 39.3 Constraints

- `RightLookupOperand = advice`
- `LookupOutput = advice`
- `rd_next = advice`

## 40. virtual_assert_eq

### 40.1 Columns

- `Rs1Idx`, `rs1_val`, `Rs2Idx`, `rs2_val`
- Lookup tuple with `EqualTable`
- `OpFlags(Assert)`

### 40.2 Variables

- `cond = (rs1_val == rs2_val)`

### 40.3 Constraints

- `LookupOutput = EqualTable(rs1_val, rs2_val)`
- Equality is enforced because `OpFlags(Assert)` requires `LookupOutput = 1`.

## 41. virtual_assert_valid_div0

### 41.1 Columns

- `Rs1Idx`, `rs1_val`, `Rs2Idx`, `rs2_val`
- Lookup tuple with `ValidDiv0Table`
- `OpFlags(Assert)`

### 41.2 Variables

- `divisor = rs1_val`
- `quotient = rs2_val`

### 41.3 Constraints

- `LookupOutput = ValidDiv0Table(divisor, quotient)` equals `1` exactly when the
  special RISC-V division corner cases (divide-by-zero and overflow) are
  satisfied.
- Since `OpFlags(Assert) = 1`, `LookupOutput` must be `1`.

## 42. virtual_change_divisor

### 42.1 Columns

- `Rs1Idx`, `dividend`, `Rs2Idx`, `divisor`
- `RdIdx`, `rd_prev`, `rd_next`
- Lookup tuple with `VirtualChangeDivisorTable`

### 42.2 Variables

- `rd_next = adjusted_divisor`, which equals `divisor` unless the inputs encode
  the overflow case `(dividend = -2^{XLEN-1}, divisor = -1)`, in which case the
  table returns `1`.

### 42.3 Constraints

- `LookupOutput = VirtualChangeDivisorTable(dividend, divisor)`
- `rd_next = LookupOutput`

## 43. virtual_assert_valid_unsigned_remainder

### 43.1 Columns

- `Rs1Idx`, `remainder`, `Rs2Idx`, `divisor`
- Lookup tuple with `ValidUnsignedRemainderTable`
- `OpFlags(Assert)`

### 43.2 Constraints

- `LookupOutput = 1` iff `divisor = 0` or `remainder < divisor`
- `OpFlags(Assert)` enforces the lookup output to be `1`.

## 44. virtual_movsign

### 44.1 Columns

- `Rs1Idx`, `value`
- `RdIdx`, `rd_prev`, `rd_next`
- Lookup tuple with `MovsignTable`

### 44.2 Constraints

- `LookupOutput` is all-zeroes if `value` is non-negative, otherwise it is all
  ones, giving the sign mask used by MULH/MULHSU sequences.

## 45. virtual_zero_extend_word

### 45.1 Columns

- `Rs1Idx`, `rs1_val`
- `RdIdx`, `rd_prev`, `rd_next`
- Lookup tuple with `LowerHalfWordTable`
- `OpFlags(AddOperands, WriteLookupOutputToRD)`

### 45.2 Constraints

- `LookupOutput` keeps the lower XLEN/2 bits of `rs1_val` and zeroes the
  top-half bits.
- `rd_next = LookupOutput`.

## 46. virtual_sign_extend_word

Same columns as above, but the lookup table sign-extends the high halfword using
`SignExtendHalfWordTable`.

## 47. virtual_assert_word_alignment

### 47.1 Columns

- `Rs1Idx`, `rs1_val`
- `Imm`
- Lookup tuple with `WordAlignmentTable`
- `OpFlags(Assert)`

### 47.2 Constraints

- `LookupOutput = 1` iff `rs1_val + Imm` is a multiple of 4
- Since `OpFlags(Assert) = 1`, unaligned accesses are ruled out.

## 48. virtual_assert_halfword_alignment

Identical to the word-alignment check, but uses `HalfwordAlignmentTable` to
require multiples of 2.

## Appendix A. RV32IM opcode mapping

Each RV32IM opcode either maps directly to one of the sections above or expands
into a short virtual sequence whose members are already described. The table
below lists the correspondence.

| Opcode         | AIR section(s)                                                                                                                                                                                                                                                                                                    |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `LUI`          | §1                                                                                                                                                                                                                                                                                                                |
| `AUIPC`        | §2                                                                                                                                                                                                                                                                                                                |
| `JAL`          | §3                                                                                                                                                                                                                                                                                                                |
| `JALR`         | §4                                                                                                                                                                                                                                                                                                                |
| `BEQ`          | §5                                                                                                                                                                                                                                                                                                                |
| `BNE`          | §6                                                                                                                                                                                                                                                                                                                |
| `BLT`          | §7                                                                                                                                                                                                                                                                                                                |
| `BGE`          | §8                                                                                                                                                                                                                                                                                                                |
| `BLTU`         | §9                                                                                                                                                                                                                                                                                                                |
| `BGEU`         | §10                                                                                                                                                                                                                                                                                                               |
| `ADDI`         | §19                                                                                                                                                                                                                                                                                                               |
| `SLTI`         | §23                                                                                                                                                                                                                                                                                                               |
| `SLTIU`        | §24                                                                                                                                                                                                                                                                                                               |
| `XORI`         | §16                                                                                                                                                                                                                                                                                                               |
| `ORI`          | §15                                                                                                                                                                                                                                                                                                               |
| `ANDI`         | §14                                                                                                                                                                                                                                                                                                               |
| `SLLI`         | `virtual_muli` (§33) multiplies `rs1` by `1 << shamt`                                                                                                                                                                                                                                                             |
| `SRLI`         | `virtual_shift_right_bitmaski` (§35) then `virtual_srli` (§37)                                                                                                                                                                                                                                                    |
| `SRAI`         | `virtual_shift_right_bitmaski` (§35) then `virtual_srai` (§38)                                                                                                                                                                                                                                                    |
| `ADD`          | §18                                                                                                                                                                                                                                                                                                               |
| `SUB`          | §20                                                                                                                                                                                                                                                                                                               |
| `SLL`          | `virtual_pow2` (§31) + `mul` (§25)                                                                                                                                                                                                                                                                                |
| `SLT`          | §21                                                                                                                                                                                                                                                                                                               |
| `SLTU`         | §22                                                                                                                                                                                                                                                                                                               |
| `XOR`          | §13                                                                                                                                                                                                                                                                                                               |
| `SRL`          | `virtual_shift_right_bitmask` (§34) + `virtual_srl` (§36)                                                                                                                                                                                                                                                         |
| `SRA`          | `virtual_shift_right_bitmask` (§34) + `virtual_sra` (§38)                                                                                                                                                                                                                                                         |
| `OR`           | §12                                                                                                                                                                                                                                                                                                               |
| `AND`          | §11                                                                                                                                                                                                                                                                                                               |
| `MUL`          | §25                                                                                                                                                                                                                                                                                                               |
| `MULH`         | Uses `virtual_movsign` (§44) to derive sign masks, followed by `mul` (§25) and `mulhu` (§26) as described in the tracer inline sequence                                                                                                                                                                           |
| `MULHSU`       | Combines `virtual_movsign` (§44), standard arithmetic ops (§11–§18), and `mulhu` (§26) per the inline sequence                                                                                                                                                                                                    |
| `MULHU`        | §26                                                                                                                                                                                                                                                                                                               |
| `DIV`, `DIVU`  | Expand into the division gadgets: `virtual_advice` (§39), `virtual_assert_valid_div0` (§41), `virtual_change_divisor` (§42), `virtual_assert_eq` (§40), and `virtual_assert_valid_unsigned_remainder` (§43) plus the arithmetic rows (§18, §25, §26) that reconstruct `dividend = quotient × divisor + remainder` |
| `REM`, `REMU`  | Same as division, but the final write uses the reconstructed remainder and the `virtual_assert_valid_unsigned_remainder` gadget                                                                                                                                                                                   |
| `LB`/`LBU`     | Alignment helpers (§48), address arithmetic (§18/§19), a base `ld` (§27) from the containing doubleword, followed by `virtual_zero_extend_word` (§45) or `virtual_sign_extend_word` (§46) to select and extend the byte                                                                                           |
| `LH`/`LHU`     | Same pattern as `LB` but relies on half-word alignment (§48) and uses the extend helpers to keep 16-bit slices                                                                                                                                                                                                    |
| `LW`           | Word-alignment check (§47), address masking (`and`/`andi`), `ld` (§27), bit shuffling via `slli`/`srl` (mapped earlier), and finally `virtual_sign_extend_word` (§46)                                                                                                                                             |
| `SB`/`SH`/`SW` | Verify alignment (sections §48/§47), load the covering word with `ld` (§27), merge the new byte/halfword with bitwise ops, and store with `sd` (§28)                                                                                                                                                              |
| `ECALL`        | §29                                                                                                                                                                                                                                                                                                               |
| `FENCE`        | §30                                                                                                                                                                                                                                                                                                               |
