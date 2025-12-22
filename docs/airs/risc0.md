# AIRS

The relations below reference helper arguments shared across the rv32im circuit:

- `CpuState(cycle, pc, mode, iCacheCycle)` orders instruction execution and carries the program counter and privilege mode.
- `Decode(pc, next_pc, rs1, rs2, rd, imm_low, imm_high, options)` links each instruction row to its decoded opcode and immediate encoding.
- `RegRead/RegWrite(idx, mode, word_addr, prev_cycle, low, high)` connect the logical register file to the physical memory bus, honoring the user/machine banks.
- `Unit(options, a, b, out0_low, out0_high, out1_low, out1_high)` invokes the memoized ALU units (add/sub, bitwise, compare, multiply, divide, shift).
- `AddU32`, `AddrSplit`, and `AddrCheck` enforce 32-bit additions and address legality checks.
- `VirtLoad`/`VirtStore` move data through the paged memory argument.
- `PhysMemRead/PhysMemWrite` touch CSRs and other machine-mode state.
- `IsZero` exposes a boolean flag proving that a 32-bit value is zero without leaking witnesses.
- One-hot selectors such as `opt_*` ensure that only the lane corresponding to the chosen opcode variant is enabled.

## 1. add

### 1.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 1.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_ADDSUB, AS_ADD)`
- `decode_opts = EncodeOptions(INST_REG, OUT_0, UNIT_ADDSUB, AS_ADD)`
- unit_out_select = 0 so rd_new uses unit_out0

### 1.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_0, UNIT_ADDSUB, AS_ADD))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_ADDSUB, AS_ADD), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 2. sub

### 2.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 2.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_ADDSUB, AS_SUB)`
- `decode_opts = EncodeOptions(INST_REG, OUT_0, UNIT_ADDSUB, AS_SUB)`
- unit_out_select = 0 so rd_new uses unit_out0

### 2.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_0, UNIT_ADDSUB, AS_SUB))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_ADDSUB, AS_SUB), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 3. xor

### 3.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 3.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_BIT, BIT_XOR)`
- `decode_opts = EncodeOptions(INST_REG, OUT_0, UNIT_BIT, BIT_XOR)`
- unit_out_select = 0 so rd_new uses unit_out0

### 3.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_0, UNIT_BIT, BIT_XOR))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_BIT, BIT_XOR), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 4. or

### 4.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 4.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_BIT, BIT_OR)`
- `decode_opts = EncodeOptions(INST_REG, OUT_0, UNIT_BIT, BIT_OR)`
- unit_out_select = 0 so rd_new uses unit_out0

### 4.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_0, UNIT_BIT, BIT_OR))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_BIT, BIT_OR), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 5. and

### 5.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 5.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_BIT, BIT_AND)`
- `decode_opts = EncodeOptions(INST_REG, OUT_0, UNIT_BIT, BIT_AND)`
- unit_out_select = 0 so rd_new uses unit_out0

### 5.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_0, UNIT_BIT, BIT_AND))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_BIT, BIT_AND), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 6. sll

### 6.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 6.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_SHIFT, SHIFT_LL)`
- `decode_opts = EncodeOptions(INST_REG, OUT_0, UNIT_SHIFT, SHIFT_LL)`
- unit_out_select = 0 so rd_new uses unit_out0

### 6.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_0, UNIT_SHIFT, SHIFT_LL))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_SHIFT, SHIFT_LL), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 7. srl

### 7.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 7.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_SHIFT, SHIFT_RL)`
- `decode_opts = EncodeOptions(INST_REG, OUT_0, UNIT_SHIFT, SHIFT_RL)`
- unit_out_select = 0 so rd_new uses unit_out0

### 7.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_0, UNIT_SHIFT, SHIFT_RL))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_SHIFT, SHIFT_RL), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 8. sra

### 8.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 8.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_SHIFT, SHIFT_RA)`
- `decode_opts = EncodeOptions(INST_REG, OUT_0, UNIT_SHIFT, SHIFT_RA)`
- unit_out_select = 0 so rd_new uses unit_out0

### 8.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_0, UNIT_SHIFT, SHIFT_RA))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_SHIFT, SHIFT_RA), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 9. slt

### 9.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 9.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_LT)`
- `decode_opts = EncodeOptions(INST_REG, OUT_0, UNIT_LT)`
- unit_out_select = 0 so rd_new uses unit_out0

### 9.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_0, UNIT_LT))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_LT), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 10. sltu

### 10.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 10.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_LT)`
- `decode_opts = EncodeOptions(INST_REG, OUT_1, UNIT_LT)`
- unit_out_select = 1 so rd_new uses unit_out1

### 10.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_1, UNIT_LT))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_LT), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 11. addi

### 11.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- imm_low
- imm_high
- rs2_field
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 11.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `rs2 = imm`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_ADDSUB, AS_ADD)`
- `decode_opts = EncodeOptions(INST_IMM, OUT_0, UNIT_ADDSUB, AS_ADD)`
- unit_out_select = 0 so rd_new uses unit_out0

### 11.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_IMM, OUT_0, UNIT_ADDSUB, AS_ADD))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_ADDSUB, AS_ADD), rs1, imm, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 12. xori

### 12.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- imm_low
- imm_high
- rs2_field
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 12.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `rs2 = imm`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_BIT, BIT_XOR)`
- `decode_opts = EncodeOptions(INST_IMM, OUT_0, UNIT_BIT, BIT_XOR)`
- unit_out_select = 0 so rd_new uses unit_out0

### 12.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_IMM, OUT_0, UNIT_BIT, BIT_XOR))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_BIT, BIT_XOR), rs1, imm, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 13. ori

### 13.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- imm_low
- imm_high
- rs2_field
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 13.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `rs2 = imm`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_BIT, BIT_OR)`
- `decode_opts = EncodeOptions(INST_IMM, OUT_0, UNIT_BIT, BIT_OR)`
- unit_out_select = 0 so rd_new uses unit_out0

### 13.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_IMM, OUT_0, UNIT_BIT, BIT_OR))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_BIT, BIT_OR), rs1, imm, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 14. andi

### 14.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- imm_low
- imm_high
- rs2_field
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 14.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `rs2 = imm`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_BIT, BIT_AND)`
- `decode_opts = EncodeOptions(INST_IMM, OUT_0, UNIT_BIT, BIT_AND)`
- unit_out_select = 0 so rd_new uses unit_out0

### 14.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_IMM, OUT_0, UNIT_BIT, BIT_AND))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_BIT, BIT_AND), rs1, imm, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 15. slli

### 15.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- imm_low
- imm_high
- rs2_field
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 15.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `rs2 = imm`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_SHIFT, SHIFT_LL)`
- `decode_opts = EncodeOptions(INST_IMM, OUT_0, UNIT_SHIFT, SHIFT_LL)`
- unit_out_select = 0 so rd_new uses unit_out0

### 15.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_IMM, OUT_0, UNIT_SHIFT, SHIFT_LL))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_SHIFT, SHIFT_LL), rs1, imm, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 16. srli

### 16.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- imm_low
- imm_high
- rs2_field
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 16.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `rs2 = imm`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_SHIFT, SHIFT_RL)`
- `decode_opts = EncodeOptions(INST_IMM, OUT_0, UNIT_SHIFT, SHIFT_RL)`
- unit_out_select = 0 so rd_new uses unit_out0

### 16.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_IMM, OUT_0, UNIT_SHIFT, SHIFT_RL))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_SHIFT, SHIFT_RL), rs1, imm, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 17. srai

### 17.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- imm_low
- imm_high
- rs2_field
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 17.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `rs2 = imm`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_SHIFT, SHIFT_RA)`
- `decode_opts = EncodeOptions(INST_IMM, OUT_0, UNIT_SHIFT, SHIFT_RA)`
- unit_out_select = 0 so rd_new uses unit_out0

### 17.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_IMM, OUT_0, UNIT_SHIFT, SHIFT_RA))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_SHIFT, SHIFT_RA), rs1, imm, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 18. slti

### 18.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- imm_low
- imm_high
- rs2_field
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 18.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `rs2 = imm`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_LT)`
- `decode_opts = EncodeOptions(INST_IMM, OUT_0, UNIT_LT)`
- unit_out_select = 0 so rd_new uses unit_out0

### 18.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_IMM, OUT_0, UNIT_LT))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_LT), rs1, imm, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 19. sltiu

### 19.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- imm_low
- imm_high
- rs2_field
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 19.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `rs2 = imm`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_LT)`
- `decode_opts = EncodeOptions(INST_IMM, OUT_1, UNIT_LT)`
- unit_out_select = 1 so rd_new uses unit_out1

### 19.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_IMM, OUT_1, UNIT_LT))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_LT), rs1, imm, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 20. lb

### 20.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- opt_lb
- opt_lh
- opt_lw
- opt_lbu
- opt_lhu
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- rs2_field
- imm_low
- imm_high
- addr_sum_low
- addr_sum_high
- addr_word
- addr_low0
- addr_low1
- addr_check_high
- virt_page
- virt_phys_page
- virt_low_byte
- virt_bit8
- virt_bit9
- virt_prev_cycle
- virt_data_low
- virt_data_high
- pick_short
- pick_short_b0
- pick_short_b1
- pick_byte
- sign_probe
- sign_bit


### 20.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `base = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `addr = base + imm = addr_sum_low + 2^16 * addr_sum_high`
- `word_addr = addr_word`
- `opt` is one-hot with `LOAD_LB` active
- `decode_opts = EncodeOptions(INST_LOAD, LOAD_LB)`
- `picked_half = pick_short` and `picked_byte = pick_byte`
- `sign_probe` feeds `sign_bit` for LB/LH

### 20.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_LOAD, LOAD_LB))`
- `opt_lb + opt_lh + opt_lw + opt_lbu + opt_lhu = 1`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

address computation
- `AddU32(rs1, imm, addr_sum_low, addr_sum_high)`
- `AddrSplit(addr_sum_low, addr_sum_high, addr_word, addr_low0, addr_low1)`
- `AddrCheck(addr_sum_low, addr_sum_high, mode) = addr_check_high`

memory channel
- `VirtLoad(addr_word, virt_prev_cycle, virt_data_low, virt_data_high)` matches the paged address data.

byte/half selection
- `pick_short` selects the upper or lower half-word of the fetched word using `addr_low1`
- `pick_byte` selects the target byte using `addr_low0`
- `sign_bit` reproduces the sign extension input for LB/LH

writeback
- `rd` receives the byte/half/word selected by `opt`; LBU/LHU clear the sign bits while LB/LH reuse `sign_bit`; LW forwards the entire word.

## 21. lh

### 21.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- opt_lb
- opt_lh
- opt_lw
- opt_lbu
- opt_lhu
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- rs2_field
- imm_low
- imm_high
- addr_sum_low
- addr_sum_high
- addr_word
- addr_low0
- addr_low1
- addr_check_high
- virt_page
- virt_phys_page
- virt_low_byte
- virt_bit8
- virt_bit9
- virt_prev_cycle
- virt_data_low
- virt_data_high
- pick_short
- pick_short_b0
- pick_short_b1
- pick_byte
- sign_probe
- sign_bit


### 21.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `base = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `addr = base + imm = addr_sum_low + 2^16 * addr_sum_high`
- `word_addr = addr_word`
- `opt` is one-hot with `LOAD_LH` active
- `decode_opts = EncodeOptions(INST_LOAD, LOAD_LH)`
- `picked_half = pick_short` and `picked_byte = pick_byte`
- `sign_probe` feeds `sign_bit` for LB/LH

### 21.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_LOAD, LOAD_LH))`
- `opt_lb + opt_lh + opt_lw + opt_lbu + opt_lhu = 1`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

address computation
- `AddU32(rs1, imm, addr_sum_low, addr_sum_high)`
- `AddrSplit(addr_sum_low, addr_sum_high, addr_word, addr_low0, addr_low1)`
- `AddrCheck(addr_sum_low, addr_sum_high, mode) = addr_check_high`

memory channel
- `VirtLoad(addr_word, virt_prev_cycle, virt_data_low, virt_data_high)` matches the paged address data.

byte/half selection
- `pick_short` selects the upper or lower half-word of the fetched word using `addr_low1`
- `pick_byte` selects the target byte using `addr_low0`
- `sign_bit` reproduces the sign extension input for LB/LH

writeback
- `rd` receives the byte/half/word selected by `opt`; LBU/LHU clear the sign bits while LB/LH reuse `sign_bit`; LW forwards the entire word.

## 22. lw

### 22.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- opt_lb
- opt_lh
- opt_lw
- opt_lbu
- opt_lhu
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- rs2_field
- imm_low
- imm_high
- addr_sum_low
- addr_sum_high
- addr_word
- addr_low0
- addr_low1
- addr_check_high
- virt_page
- virt_phys_page
- virt_low_byte
- virt_bit8
- virt_bit9
- virt_prev_cycle
- virt_data_low
- virt_data_high
- pick_short
- pick_short_b0
- pick_short_b1
- pick_byte
- sign_probe
- sign_bit


### 22.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `base = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `addr = base + imm = addr_sum_low + 2^16 * addr_sum_high`
- `word_addr = addr_word`
- `opt` is one-hot with `LOAD_LW` active
- `decode_opts = EncodeOptions(INST_LOAD, LOAD_LW)`
- `picked_half = pick_short` and `picked_byte = pick_byte`
- `sign_probe` feeds `sign_bit` for LB/LH

### 22.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_LOAD, LOAD_LW))`
- `opt_lb + opt_lh + opt_lw + opt_lbu + opt_lhu = 1`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

address computation
- `AddU32(rs1, imm, addr_sum_low, addr_sum_high)`
- `AddrSplit(addr_sum_low, addr_sum_high, addr_word, addr_low0, addr_low1)`
- `AddrCheck(addr_sum_low, addr_sum_high, mode) = addr_check_high`

memory channel
- `VirtLoad(addr_word, virt_prev_cycle, virt_data_low, virt_data_high)` matches the paged address data.

byte/half selection
- `pick_short` selects the upper or lower half-word of the fetched word using `addr_low1`
- `pick_byte` selects the target byte using `addr_low0`
- `sign_bit` reproduces the sign extension input for LB/LH

writeback
- `rd` receives the byte/half/word selected by `opt`; LBU/LHU clear the sign bits while LB/LH reuse `sign_bit`; LW forwards the entire word.

## 23. lbu

### 23.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- opt_lb
- opt_lh
- opt_lw
- opt_lbu
- opt_lhu
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- rs2_field
- imm_low
- imm_high
- addr_sum_low
- addr_sum_high
- addr_word
- addr_low0
- addr_low1
- addr_check_high
- virt_page
- virt_phys_page
- virt_low_byte
- virt_bit8
- virt_bit9
- virt_prev_cycle
- virt_data_low
- virt_data_high
- pick_short
- pick_short_b0
- pick_short_b1
- pick_byte
- sign_probe
- sign_bit


### 23.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `base = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `addr = base + imm = addr_sum_low + 2^16 * addr_sum_high`
- `word_addr = addr_word`
- `opt` is one-hot with `LOAD_LBU` active
- `decode_opts = EncodeOptions(INST_LOAD, LOAD_LBU)`
- `picked_half = pick_short` and `picked_byte = pick_byte`
- `sign_probe` feeds `sign_bit` for LB/LH

### 23.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_LOAD, LOAD_LBU))`
- `opt_lb + opt_lh + opt_lw + opt_lbu + opt_lhu = 1`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

address computation
- `AddU32(rs1, imm, addr_sum_low, addr_sum_high)`
- `AddrSplit(addr_sum_low, addr_sum_high, addr_word, addr_low0, addr_low1)`
- `AddrCheck(addr_sum_low, addr_sum_high, mode) = addr_check_high`

memory channel
- `VirtLoad(addr_word, virt_prev_cycle, virt_data_low, virt_data_high)` matches the paged address data.

byte/half selection
- `pick_short` selects the upper or lower half-word of the fetched word using `addr_low1`
- `pick_byte` selects the target byte using `addr_low0`
- `sign_bit` reproduces the sign extension input for LB/LH

writeback
- `rd` receives the byte/half/word selected by `opt`; LBU/LHU clear the sign bits while LB/LH reuse `sign_bit`; LW forwards the entire word.

## 24. lhu

### 24.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- opt_lb
- opt_lh
- opt_lw
- opt_lbu
- opt_lhu
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- rs2_field
- imm_low
- imm_high
- addr_sum_low
- addr_sum_high
- addr_word
- addr_low0
- addr_low1
- addr_check_high
- virt_page
- virt_phys_page
- virt_low_byte
- virt_bit8
- virt_bit9
- virt_prev_cycle
- virt_data_low
- virt_data_high
- pick_short
- pick_short_b0
- pick_short_b1
- pick_byte
- sign_probe
- sign_bit


### 24.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `base = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `addr = base + imm = addr_sum_low + 2^16 * addr_sum_high`
- `word_addr = addr_word`
- `opt` is one-hot with `LOAD_LHU` active
- `decode_opts = EncodeOptions(INST_LOAD, LOAD_LHU)`
- `picked_half = pick_short` and `picked_byte = pick_byte`
- `sign_probe` feeds `sign_bit` for LB/LH

### 24.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_LOAD, LOAD_LHU))`
- `opt_lb + opt_lh + opt_lw + opt_lbu + opt_lhu = 1`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

address computation
- `AddU32(rs1, imm, addr_sum_low, addr_sum_high)`
- `AddrSplit(addr_sum_low, addr_sum_high, addr_word, addr_low0, addr_low1)`
- `AddrCheck(addr_sum_low, addr_sum_high, mode) = addr_check_high`

memory channel
- `VirtLoad(addr_word, virt_prev_cycle, virt_data_low, virt_data_high)` matches the paged address data.

byte/half selection
- `pick_short` selects the upper or lower half-word of the fetched word using `addr_low1`
- `pick_byte` selects the target byte using `addr_low0`
- `sign_bit` reproduces the sign extension input for LB/LH

writeback
- `rd` receives the byte/half/word selected by `opt`; LBU/LHU clear the sign bits while LB/LH reuse `sign_bit`; LW forwards the entire word.

## 25. sb

### 25.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- opt_sb
- opt_sh
- opt_sw
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_field
- imm_low
- imm_high
- addr_sum_low
- addr_sum_high
- addr_word
- addr_low0
- addr_low1
- addr_check_high
- virt_page
- virt_phys_page
- virt_low_byte
- virt_bit8
- virt_bit9
- virt_prev_cycle
- virt_prev_data_low
- virt_prev_data_high
- virt_data_low
- virt_data_high
- pick_short
- pick_short_b0
- pick_short_b1
- pick_byte
- rs2_byte0
- rs2_byte1
- merged_short


### 25.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `base = rs1_low + 2^16 * rs1_high`
- `data = rs2_low + 2^16 * rs2_high`
- `imm = imm_low + 2^16 * imm_high`
- `addr = base + imm = addr_sum_low + 2^16 * addr_sum_high`
- `word_addr = addr_word`
- `opt` is one-hot with `STORE_SB` active
- `decode_opts = EncodeOptions(INST_STORE, STORE_SB)`
- `merged_short` is the rewritten half-word used for SB/SH

### 25.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rd_field, rs2_idx, imm_low, imm_high, EncodeOptions(INST_STORE, STORE_SB))`
- `opt_sb + opt_sh + opt_sw = 1`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`

address computation
- `AddU32(rs1, imm, addr_sum_low, addr_sum_high)`
- `AddrSplit(addr_sum_low, addr_sum_high, addr_word, addr_low0, addr_low1)`
- `AddrCheck(addr_sum_low, addr_sum_high, mode) = addr_check_high`

memory channel
- `VirtStore(addr_word, virt_prev_cycle, virt_prev_data_low, virt_prev_data_high, virt_data_low, virt_data_high)`

byte/half merge
- `pick_short` selects the half-word that is being overwritten
- `merged_short` flips the correct byte(s) from `data` into the touched half-word depending on `addr_low0`
- complete words forward the `rs2` value
- untouched bytes are kept via the previous data witnesses

## 26. sh

### 26.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- opt_sb
- opt_sh
- opt_sw
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_field
- imm_low
- imm_high
- addr_sum_low
- addr_sum_high
- addr_word
- addr_low0
- addr_low1
- addr_check_high
- virt_page
- virt_phys_page
- virt_low_byte
- virt_bit8
- virt_bit9
- virt_prev_cycle
- virt_prev_data_low
- virt_prev_data_high
- virt_data_low
- virt_data_high
- pick_short
- pick_short_b0
- pick_short_b1
- pick_byte
- rs2_byte0
- rs2_byte1
- merged_short


### 26.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `base = rs1_low + 2^16 * rs1_high`
- `data = rs2_low + 2^16 * rs2_high`
- `imm = imm_low + 2^16 * imm_high`
- `addr = base + imm = addr_sum_low + 2^16 * addr_sum_high`
- `word_addr = addr_word`
- `opt` is one-hot with `STORE_SH` active
- `decode_opts = EncodeOptions(INST_STORE, STORE_SH)`
- `merged_short` is the rewritten half-word used for SB/SH

### 26.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rd_field, rs2_idx, imm_low, imm_high, EncodeOptions(INST_STORE, STORE_SH))`
- `opt_sb + opt_sh + opt_sw = 1`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`

address computation
- `AddU32(rs1, imm, addr_sum_low, addr_sum_high)`
- `AddrSplit(addr_sum_low, addr_sum_high, addr_word, addr_low0, addr_low1)`
- `AddrCheck(addr_sum_low, addr_sum_high, mode) = addr_check_high`

memory channel
- `VirtStore(addr_word, virt_prev_cycle, virt_prev_data_low, virt_prev_data_high, virt_data_low, virt_data_high)`

byte/half merge
- `pick_short` selects the half-word that is being overwritten
- `merged_short` flips the correct byte(s) from `data` into the touched half-word depending on `addr_low0`
- complete words forward the `rs2` value
- untouched bytes are kept via the previous data witnesses

## 27. sw

### 27.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- opt_sb
- opt_sh
- opt_sw
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_field
- imm_low
- imm_high
- addr_sum_low
- addr_sum_high
- addr_word
- addr_low0
- addr_low1
- addr_check_high
- virt_page
- virt_phys_page
- virt_low_byte
- virt_bit8
- virt_bit9
- virt_prev_cycle
- virt_prev_data_low
- virt_prev_data_high
- virt_data_low
- virt_data_high
- pick_short
- pick_short_b0
- pick_short_b1
- pick_byte
- rs2_byte0
- rs2_byte1
- merged_short


### 27.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `base = rs1_low + 2^16 * rs1_high`
- `data = rs2_low + 2^16 * rs2_high`
- `imm = imm_low + 2^16 * imm_high`
- `addr = base + imm = addr_sum_low + 2^16 * addr_sum_high`
- `word_addr = addr_word`
- `opt` is one-hot with `STORE_SW` active
- `decode_opts = EncodeOptions(INST_STORE, STORE_SW)`
- `merged_short` is the rewritten half-word used for SB/SH

### 27.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rd_field, rs2_idx, imm_low, imm_high, EncodeOptions(INST_STORE, STORE_SW))`
- `opt_sb + opt_sh + opt_sw = 1`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`

address computation
- `AddU32(rs1, imm, addr_sum_low, addr_sum_high)`
- `AddrSplit(addr_sum_low, addr_sum_high, addr_word, addr_low0, addr_low1)`
- `AddrCheck(addr_sum_low, addr_sum_high, mode) = addr_check_high`

memory channel
- `VirtStore(addr_word, virt_prev_cycle, virt_prev_data_low, virt_prev_data_high, virt_data_low, virt_data_high)`

byte/half merge
- `pick_short` selects the half-word that is being overwritten
- `merged_short` flips the correct byte(s) from `data` into the touched half-word depending on `addr_low0`
- complete words forward the `rs2` value
- untouched bytes are kept via the previous data witnesses

## 28. beq

### 28.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_field
- imm_low
- imm_high
- opt_out
- branch_on_nz
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high
- is_out_zero
- sum_pc_low
- sum_pc_high
- new_pc_low
- new_pc_high


### 28.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `imm = imm_low + 2^16 * imm_high`
- `sum_pc = sum_pc_low + 2^16 * sum_pc_high`
- `new_pc = new_pc_low + 2^16 * new_pc_high`
- `unit_opts = EncodeOptions(UNIT_ADDSUB, AS_SUB)`
- `decode_opts = EncodeOptions(INST_BRANCH, BR_Z, OUT_0, UNIT_ADDSUB, AS_SUB)`
- `unit_out_select = 0 so rd_new uses unit_out0`
- `branch_on_nz = 0 so the jump occurs when the selected unit output is zero`

### 28.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, new_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_field, imm_low, imm_high, EncodeOptions(INST_BRANCH, BR_Z, OUT_0, UNIT_ADDSUB, AS_SUB))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_ADDSUB, AS_SUB), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `branch_operand = (1 - unit_out_select) * unit_out0 + unit_out_select * unit_out1`
- `is_out_zero` proves whether `branch_operand` equals zero

branch target
- `AddU32(pc, imm, sum_pc_low, sum_pc_high)`
- `new_pc` equals `sum_pc` when the branch fires and `next_pc` otherwise; the `branch_on_nz` bit flips the condition between zero and non-zero.

## 29. bne

### 29.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_field
- imm_low
- imm_high
- opt_out
- branch_on_nz
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high
- is_out_zero
- sum_pc_low
- sum_pc_high
- new_pc_low
- new_pc_high


### 29.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `imm = imm_low + 2^16 * imm_high`
- `sum_pc = sum_pc_low + 2^16 * sum_pc_high`
- `new_pc = new_pc_low + 2^16 * new_pc_high`
- `unit_opts = EncodeOptions(UNIT_ADDSUB, AS_SUB)`
- `decode_opts = EncodeOptions(INST_BRANCH, BR_NZ, OUT_0, UNIT_ADDSUB, AS_SUB)`
- `unit_out_select = 0 so rd_new uses unit_out0`
- `branch_on_nz = 1 so the jump occurs when the selected unit output is non-zero`

### 29.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, new_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_field, imm_low, imm_high, EncodeOptions(INST_BRANCH, BR_NZ, OUT_0, UNIT_ADDSUB, AS_SUB))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_ADDSUB, AS_SUB), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `branch_operand = (1 - unit_out_select) * unit_out0 + unit_out_select * unit_out1`
- `is_out_zero` proves whether `branch_operand` equals zero

branch target
- `AddU32(pc, imm, sum_pc_low, sum_pc_high)`
- `new_pc` equals `sum_pc` when the branch fires and `next_pc` otherwise; the `branch_on_nz` bit flips the condition between zero and non-zero.

## 30. blt

### 30.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_field
- imm_low
- imm_high
- opt_out
- branch_on_nz
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high
- is_out_zero
- sum_pc_low
- sum_pc_high
- new_pc_low
- new_pc_high


### 30.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `imm = imm_low + 2^16 * imm_high`
- `sum_pc = sum_pc_low + 2^16 * sum_pc_high`
- `new_pc = new_pc_low + 2^16 * new_pc_high`
- `unit_opts = EncodeOptions(UNIT_LT)`
- `decode_opts = EncodeOptions(INST_BRANCH, BR_NZ, OUT_0, UNIT_LT)`
- `unit_out_select = 0 so rd_new uses unit_out0`
- `branch_on_nz = 1 so the jump occurs when the selected unit output is non-zero`

### 30.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, new_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_field, imm_low, imm_high, EncodeOptions(INST_BRANCH, BR_NZ, OUT_0, UNIT_LT))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_LT), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `branch_operand = (1 - unit_out_select) * unit_out0 + unit_out_select * unit_out1`
- `is_out_zero` proves whether `branch_operand` equals zero

branch target
- `AddU32(pc, imm, sum_pc_low, sum_pc_high)`
- `new_pc` equals `sum_pc` when the branch fires and `next_pc` otherwise; the `branch_on_nz` bit flips the condition between zero and non-zero.

## 31. bge

### 31.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_field
- imm_low
- imm_high
- opt_out
- branch_on_nz
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high
- is_out_zero
- sum_pc_low
- sum_pc_high
- new_pc_low
- new_pc_high


### 31.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `imm = imm_low + 2^16 * imm_high`
- `sum_pc = sum_pc_low + 2^16 * sum_pc_high`
- `new_pc = new_pc_low + 2^16 * new_pc_high`
- `unit_opts = EncodeOptions(UNIT_LT)`
- `decode_opts = EncodeOptions(INST_BRANCH, BR_Z, OUT_0, UNIT_LT)`
- `unit_out_select = 0 so rd_new uses unit_out0`
- `branch_on_nz = 0 so the jump occurs when the selected unit output is zero`

### 31.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, new_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_field, imm_low, imm_high, EncodeOptions(INST_BRANCH, BR_Z, OUT_0, UNIT_LT))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_LT), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `branch_operand = (1 - unit_out_select) * unit_out0 + unit_out_select * unit_out1`
- `is_out_zero` proves whether `branch_operand` equals zero

branch target
- `AddU32(pc, imm, sum_pc_low, sum_pc_high)`
- `new_pc` equals `sum_pc` when the branch fires and `next_pc` otherwise; the `branch_on_nz` bit flips the condition between zero and non-zero.

## 32. bltu

### 32.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_field
- imm_low
- imm_high
- opt_out
- branch_on_nz
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high
- is_out_zero
- sum_pc_low
- sum_pc_high
- new_pc_low
- new_pc_high


### 32.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `imm = imm_low + 2^16 * imm_high`
- `sum_pc = sum_pc_low + 2^16 * sum_pc_high`
- `new_pc = new_pc_low + 2^16 * new_pc_high`
- `unit_opts = EncodeOptions(UNIT_LT)`
- `decode_opts = EncodeOptions(INST_BRANCH, BR_NZ, OUT_1, UNIT_LT)`
- `unit_out_select = 1 so rd_new uses unit_out1`
- `branch_on_nz = 1 so the jump occurs when the selected unit output is non-zero`

### 32.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, new_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_field, imm_low, imm_high, EncodeOptions(INST_BRANCH, BR_NZ, OUT_1, UNIT_LT))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_LT), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `branch_operand = (1 - unit_out_select) * unit_out0 + unit_out_select * unit_out1`
- `is_out_zero` proves whether `branch_operand` equals zero

branch target
- `AddU32(pc, imm, sum_pc_low, sum_pc_high)`
- `new_pc` equals `sum_pc` when the branch fires and `next_pc` otherwise; the `branch_on_nz` bit flips the condition between zero and non-zero.

## 33. bgeu

### 33.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_field
- imm_low
- imm_high
- opt_out
- branch_on_nz
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high
- is_out_zero
- sum_pc_low
- sum_pc_high
- new_pc_low
- new_pc_high


### 33.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `imm = imm_low + 2^16 * imm_high`
- `sum_pc = sum_pc_low + 2^16 * sum_pc_high`
- `new_pc = new_pc_low + 2^16 * new_pc_high`
- `unit_opts = EncodeOptions(UNIT_LT)`
- `decode_opts = EncodeOptions(INST_BRANCH, BR_Z, OUT_1, UNIT_LT)`
- `unit_out_select = 1 so rd_new uses unit_out1`
- `branch_on_nz = 0 so the jump occurs when the selected unit output is zero`

### 33.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, new_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_field, imm_low, imm_high, EncodeOptions(INST_BRANCH, BR_Z, OUT_1, UNIT_LT))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`

unit wiring
- `- Unit(EncodeOptions(UNIT_LT), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `branch_operand = (1 - unit_out_select) * unit_out0 + unit_out_select * unit_out1`
- `is_out_zero` proves whether `branch_operand` equals zero

branch target
- `AddU32(pc, imm, sum_pc_low, sum_pc_high)`
- `new_pc` equals `sum_pc` when the branch fires and `next_pc` otherwise; the `branch_on_nz` bit flips the condition between zero and non-zero.

## 34. jal

### 34.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_field
- rs2_field
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- imm_low
- imm_high
- sum_pc_low
- sum_pc_high


### 34.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `imm = imm_low + 2^16 * imm_high`
- `sum_pc = sum_pc_low + 2^16 * sum_pc_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `decode_opts = EncodeOptions(INST_JAL)`

### 34.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, sum_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_field, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_JAL))`

writeback
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rd_low + 2^16 * rd_high = next_pc`
- `sum_pc` uses `AddU32(pc, imm)` for the jump target

## 35. jalr

### 35.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_field
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- imm_low
- imm_high
- sum_pc_low
- sum_pc_high


### 35.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `imm = imm_low + 2^16 * imm_high`
- `sum_pc = sum_pc_low + 2^16 * sum_pc_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `decode_opts = EncodeOptions(INST_JALR)`

### 35.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, sum_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_JALR))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

jump target
- `AddU32(rs1, imm, sum_pc_low, sum_pc_high)` forms the indirect target
- `rd_low + 2^16 * rd_high = next_pc`

## 36. lui

### 36.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_field
- rs2_field
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero


### 36.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `decode_opts = EncodeOptions(INST_LUI)`

### 36.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_field, rs2_field, rd_idx, rd_low, rd_high, EncodeOptions(INST_LUI))`

writeback
- `rd` writes the literal provided by the decoder (upper immediate)
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

## 37. auipc

### 37.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_field
- rs2_field
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- imm_low
- imm_high
- sum_pc_low
- sum_pc_high


### 37.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `imm = imm_low + 2^16 * imm_high`
- `sum_pc = sum_pc_low + 2^16 * sum_pc_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `decode_opts = EncodeOptions(INST_AUIPC)`

### 37.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_field, rs2_field, rd_idx, imm_low, imm_high, EncodeOptions(INST_AUIPC))`

writeback
- `AddU32(pc, imm, sum_pc_low, sum_pc_high)`
- `rd` stores `sum_pc`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`

## 38. mul

### 38.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 38.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_MUL, MUL_SS)`
- `decode_opts = EncodeOptions(INST_REG, OUT_0, UNIT_MUL, MUL_SS)`
- unit_out_select = 0 so rd_new uses unit_out0

### 38.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_0, UNIT_MUL, MUL_SS))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_MUL, MUL_SS), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 39. mulh

### 39.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 39.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_MUL, MUL_SS)`
- `decode_opts = EncodeOptions(INST_REG, OUT_1, UNIT_MUL, MUL_SS)`
- unit_out_select = 1 so rd_new uses unit_out1

### 39.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_1, UNIT_MUL, MUL_SS))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_MUL, MUL_SS), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 40. mulhsu

### 40.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 40.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_MUL, MUL_SU)`
- `decode_opts = EncodeOptions(INST_REG, OUT_1, UNIT_MUL, MUL_SU)`
- unit_out_select = 1 so rd_new uses unit_out1

### 40.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_1, UNIT_MUL, MUL_SU))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_MUL, MUL_SU), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 41. mulhu

### 41.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 41.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_MUL, MUL_UU)`
- `decode_opts = EncodeOptions(INST_REG, OUT_1, UNIT_MUL, MUL_UU)`
- unit_out_select = 1 so rd_new uses unit_out1

### 41.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_1, UNIT_MUL, MUL_UU))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_MUL, MUL_UU), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 42. div

### 42.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 42.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_DIV, DIV_S)`
- `decode_opts = EncodeOptions(INST_REG, OUT_0, UNIT_DIV, DIV_S)`
- unit_out_select = 0 so rd_new uses unit_out0

### 42.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_0, UNIT_DIV, DIV_S))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_DIV, DIV_S), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 43. divu

### 43.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 43.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_DIV, DIV_U)`
- `decode_opts = EncodeOptions(INST_REG, OUT_0, UNIT_DIV, DIV_U)`
- unit_out_select = 0 so rd_new uses unit_out0

### 43.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_0, UNIT_DIV, DIV_U))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_DIV, DIV_U), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 44. rem

### 44.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 44.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_DIV, DIV_S)`
- `decode_opts = EncodeOptions(INST_REG, OUT_1, UNIT_DIV, DIV_S)`
- unit_out_select = 1 so rd_new uses unit_out1

### 44.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_1, UNIT_DIV, DIV_S))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_DIV, DIV_S), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 45. remu

### 45.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- rs1_word_addr
- rs1_prev_cycle
- rs1_low
- rs1_high
- rs1_idx
- rs2_word_addr
- rs2_prev_cycle
- rs2_low
- rs2_high
- rs2_idx
- rs2_same_reg
- rd_word_addr
- rd_prev_cycle
- rd_prev_low
- rd_prev_high
- rd_low
- rd_high
- rd_idx
- rd_is_zero
- unit_opts
- unit_out_select
- unit_out0_low
- unit_out0_high
- unit_out1_low
- unit_out1_high


### 45.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `rs1 = rs1_low + 2^16 * rs1_high`
- `rs2 = rs2_low + 2^16 * rs2_high`
- `rd_prev = rd_prev_low + 2^16 * rd_prev_high`
- `rd_new = rd_low + 2^16 * rd_high`
- `unit_opts = EncodeOptions(UNIT_DIV, DIV_U)`
- `decode_opts = EncodeOptions(INST_REG, OUT_1, UNIT_DIV, DIV_U)`
- unit_out_select = 1 so rd_new uses unit_out1

### 45.3 Constraints

program order
- `- CpuState(cycle, pc, mode, i_cache_cycle)`
- `+ CpuState(cycle + 1, next_pc, mode, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, rs1_idx, rs2_idx, rd_idx, 0, 0, EncodeOptions(INST_REG, OUT_1, UNIT_DIV, DIV_U))`

register accesses
- `- RegRead(rs1_idx, mode, rs1_word_addr, rs1_prev_cycle, rs1_low, rs1_high)`
- `- RegRead(rs2_idx, mode, rs2_word_addr, rs2_prev_cycle, rs2_low, rs2_high)`
- `- RegWrite(rd_idx, mode, rd_word_addr, rd_prev_cycle, rd_prev_low, rd_prev_high, rd_low, rd_high)`
- `rs2_same_reg * (rs1_idx - rs2_idx) = 0` (single-read fast path flag)

unit wiring
- `- Unit(EncodeOptions(UNIT_DIV, DIV_U), rs1, rs2, unit_out0_low, unit_out0_high, unit_out1_low, unit_out1_high)`
- `rd_low + 2^16 * rd_high = (1 - unit_out_select) * (unit_out0_low + 2^16 * unit_out0_high)` + `unit_out_select * (unit_out1_low + 2^16 * unit_out1_high)`

address remapping
- `SourceReg`/`DestReg` tie logical indices to physical word addresses and redirect x0 writes via `rd_is_zero`.

## 46. ecall

### 46.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- save_pc_word
- save_pc_prev_cycle
- save_pc_prev_low
- save_pc_prev_high
- save_pc_low
- save_pc_high
- dispatch_word
- dispatch_prev_cycle
- dispatch_low
- dispatch_high


### 46.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `save_pc = save_pc_low + 2^16 * save_pc_high`
- `dispatch = dispatch_low + 2^16 * dispatch_high`
- `decode_opts = EncodeOptions(INST_ECALL)`

### 46.3 Constraints

control transfer
- `- CpuState(cycle, pc, MODE_USER, i_cache_cycle)`
- `+ CpuState(cycle + 1, dispatch, MODE_MACHINE, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, 0, 0, 0, 0, 0, EncodeOptions(INST_ECALL))`

csr wiring
- `PhysMemWrite` stores `pc` into `MEPC`
- `PhysMemRead` fetches the ecall dispatch pointer (`MTVEC` equivalent)`
- CSR addresses depend on the global `v2Compat` flag

## 47. mret

### 47.1 Columns

- cycle
- pc_low
- pc_high
- next_pc_low
- next_pc_high
- mode
- i_cache_cycle
- load_cycle
- read_pc_word
- read_pc_prev_cycle
- read_pc_low
- read_pc_high
- to_add
- sum_pc_low
- sum_pc_high


### 47.2 Variables

- `pc = pc_low + 2^16 * pc_high`
- `next_pc = next_pc_low + 2^16 * next_pc_high`
- `saved_pc = read_pc_low + 2^16 * read_pc_high`
- `sum_pc = sum_pc_low + 2^16 * sum_pc_high`
- `decode_opts = EncodeOptions(INST_MRET)`

### 47.3 Constraints

control transfer
- `- CpuState(cycle, pc, MODE_MACHINE, i_cache_cycle)`
- `+ CpuState(cycle + 1, sum_pc, MODE_USER, i_cache_cycle)`

decoding
- `- Decode(pc, next_pc, 0, 2, 0, 770, 0, EncodeOptions(INST_MRET))`

csr wiring
- `PhysMemRead` loads `MEPC`
- `to_add` injects the compatibility offset (`4` when `v2Compat` is set)`
- `AddU32(saved_pc, to_add, sum_pc_low, sum_pc_high)` provides the resumed PC
