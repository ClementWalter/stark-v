# AIRS

## 1. Base ALU Reg(add/sub/xor/or/and)

### 1.1 Columns

- pc
- clk
- in_place_flag_1
- in_place_flag_2

- rd_idx
- rd_prev_clk
- rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3.
- a_0, a_1, a_2, a_3 — limbs of the value written to `rd`.

- rs1_idx
- rs1_prev_clk
- b_0, b_1, b_2, b_3 — limbs of `rs1`.

- rs2_idx
- rs2_prev_clk
- c_0, c_1, c_2, c_3 — limbs of `rs2`

- opcode_add_flag
- opcode_sub_flag
- opcode_xor_flag
- opcode_or_flag
- opcode_and_flag

### 1.2 Variables

- `is_valid = opcode_add_flag + opcode_sub_flag + opcode_xor_flag + opcode_or_flag + opcode_and_flag`.
- `carry_add[i] = (b_i + c_i + carry_add[i - 1] - a_i) / 2^N_BITS_PER_BYTE` with
  `carry_add[-1] = 0`.
- `carry_sub[i] = (a_i + c_i - b_i + carry_sub[i - 1]) / 2^N_BITS_PER_BYTE` with
  `carry_sub[-1] = 0`.
- `is_bitwise = opcode_xor_flag + opcode_or_flag + opcode_and_flag`
- `bitwise = opcode_xor_flag + 2 * opcode_or_flag + 3 * opcode_and_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `rs_idx_diff = rs1_idx - rs2_idx`
- `rd_idx_diff_1 = rd_idx - rs1_idx`
- `rd_idx_diff_2 = rd_idx - rs2_idx`

### 1.3 Constraints

`is_valid`, `opcode_*_flags` and `in_place_flags` are booleans

- `opcode_*_flag * (1 - opcode_*_flag)`
- `is_valid * (1 - is_valid)`
- `in_place_flag_i * (1 - in_place_flag_i)`

if in-place flag is 1 then register diff (or one of register diffs) is 0

- `in_place_flag_1 * rs_idx_diff`
- `in_place_flag_2 * rd_idx_diff_1 * rd_idx_diff_2`

read instruction from the Program segment

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, rs2_idx)`

registers update

- `- is_valid * RegsImm(pc, clk)`
- `+ is_valid * RegsImm(pc + 4, clk + 1)`

read from rs1

- `- is_valid * RegsRW(rs1_idx, rs1_prev_clk, b_0, b_1, b_2, b_3)`
- `+ is_valid * RegsRW(rs1_idx, clk, b_0, b_1, b_2, b_3)`
- `- RC_20(clk - rs1_prev_clk - is_valid)`

read from rs2

- `- is_valid * RegsRW(rs2_idx, rs2_prev_clk, c_0, c_1, c_2, c_3)`
- `+ is_valid * RegsRW(rs2_idx, clk, c_0, c_1, c_2, c_3)`
- `- (1 - in_place_flag_1) * RC_20(clk - rs2_prev_clk - is_valid)`
- `in_place_flag_1 * (clk - rs2_prev_clk)`

check carries

- `opcode_add_flag * carry_add[i] * (1 - carry_add[i])`
- `opcode_sub_flag * carry_sub[i] * (1 - carry_sub[i])`

perform bitwise operation

- `- is_bitwise * Bitwise(b_0, c_0, a_0, bitwise)`
- `- is_bitwise * Bitwise(b_1, c_1, a_1, bitwise)`
- `- is_bitwise * Bitwise(b_2, c_2, a_2, bitwise)`
- `- is_bitwise * Bitwise(b_3, c_3, a_3, bitwise)`

range check a (redundant for bitwise)

- `- RC_8_8(a_0, a_1)`
- `- RC_8_8(a_2, a_3)`

write to rd

- `- is_valid * RegsRW(rd_idx, rd_prev_clk, rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3)`
- `+ is_valid * RegsRW(rd_idx, clk, a_0, a_1, a_2, a_3)`
- `- (1 - in_place_flag_2) * RC_20(clk - rd_prev_clk - is_valid)`
- `in_place_flag_2 * (clk - rd_prev_clk)`

## 2. Base ALU Imm(addi/subi/xori/ori/andi)

### 2.1 Columns

- pc
- clk
- in_place_flag

- rd_idx
- rd_prev_clk
- rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3.
- a_0, a_1, a_2, a_3 — limbs of the value written to `rd`.

- rs1_idx
- rs1_prev_clk
- b_0, b_1, b_2, b_3 — limbs of `rs1`.

- imm_0 (imm[0:7])
- imm_1 (imm[8:10])
- imm_msb (imm[11])
- sext_imm_0
- sext_imm_1
- sext_imm_2
- sext_imm_3

- opcode_add_flag
- opcode_sub_flag
- opcode_xor_flag
- opcode_or_flag
- opcode_and_flag

### 2.2 Variables

- `SEXT_CST_i = le_bytes(2^32 - 2^12)[i]`
- `is_valid = opcode_add_flag + opcode_sub_flag + opcode_xor_flag + opcode_or_flag + opcode_and_flag`.
- `carry_sext[0] = (imm_0 + SEXT_CST_0 - sext_imm_0) / 2^N_BITS_PER_BYTE`
- `carry_sext[1] = (imm_1 + 2^3 * imm_msb + SEXT_CST_1 + carry_sext[0] - sext_imm_1) / 2^N_BITS_PER_BYTE`
- `carry_sext[2] = (SEXT_CST_2 + carry_sext[1] - sext_imm_2) / 2^N_BITS_PER_BYTE`
- `carry_sext[3] = (SEXT_CST_3 + carry_sext[2] - sext_imm_3) / 2^N_BITS_PER_BYTE`
- `carry_add[i] = (b_i + sext_imm_i + carry_add[i - 1] - a_i) / 2^N_BITS_PER_BYTE`
  with `carry_add[-1] = 0`
- `carry_sub[i] = (a_i + sext_imm_i - b_i + carry_sub[i - 1]) / 2^N_BITS_PER_BYTE`
  with `carry_sub[-1] = 0`
- `is_bitwise = opcode_xor_flag + opcode_or_flag + opcode_and_flag`
- `bitwise = opcode_xor_flag + 2 * opcode_or_flag + 3 * opcode_and_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `imm = imm_0 + imm_1 * 2^8 + imm_msb * 2^11`
- `r_idx_diff = rd_idx - rs1_idx`

### 2.3 Constraints

`is_valid`, `opcode_*_flags`, `in_place_flag` and `imm_msb` are booleans

- `opcode_*_flag * (1 - opcode_*_flag)`
- `is_valid * (1 - is_valid)`
- `in_place_flag * (1 - in_place_flag)`

read instruction from the Program segment

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, imm)`

range check imm and sext_imm

- `- RC_8_3(imm_0, imm_1)`
- `imm_msb * (1 - imm_msb)`
- `- RC_8_8(sext_imm_0, sext_imm_1)`
- `- RC_8_8(sext_imm_2, sext_imm_3)`

sext_imm is imm sign-extended

- `imm_msb * carry_sext[i] * (1 - carry_sext[i])`
- `(1 - imm_msb) * (imm_0 - sext_imm_0)`
- `(1 - imm_msb) * (imm_1 - sext_imm_1)`
- `(1 - imm_msb) * (0 - sext_imm_2)`
- `(1 - imm_msb) * (0 - sext_imm_3)`

registers update

- `- is_valid * RegsImm(pc, clk)`
- `+ is_valid * RegsImm(pc + 4, clk + 1)`

read from rs1

- `- is_valid * RegsRW(rs1_idx, rs1_prev_clk, b_0, b_1, b_2, b_3)`
- `+ is_valid * RegsRW(rs1_idx, clk, b_0, b_1, b_2, b_3)`
- `- RC_20(clk - rs1_prev_clk - is_valid)`

check carries

- `opcode_add_flag * carry_add[i] * (1 - carry_add[i])`
- `opcode_sub_flag * carry_sub[i] * (1 - carry_sub[i])`

perform bitwise operation

- `- is_bitwise * Bitwise(b_0, sext_imm_0, a_0, bitwise)`
- `- is_bitwise * Bitwise(b_1, sext_imm_1, a_1, bitwise)`
- `- is_bitwise * Bitwise(b_2, sext_imm_2, a_2, bitwise)`
- `- is_bitwise * Bitwise(b_3, sext_imm_3, a_3, bitwise)`

range check a (redundant for bitwise)

- `- RC_8_8(a_0, a_1)`
- `- RC_8_8(a_2, a_3)`

if in-place flag is 1 then rd_idx == rs1_idx

- `in_place_flag * r_idx_diff`

write to rd

- `- is_valid * RegsRW(rd_idx, rd_prev_clk, rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3)`
- `+ is_valid * RegsRW(rd_idx, clk, a_0, a_1, a_2, a_3)`
- `- (1 - in_place_flag) * RC_20(clk - rd_prev_clk - is_valid)`
- `in_place_flag * (clk - rd_prev_clk)`
