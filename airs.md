# AIRS

## 1. Base ALU Reg (add/sub/xor/or/and)

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

- `enabler = opcode_add_flag + opcode_sub_flag + opcode_xor_flag + opcode_or_flag + opcode_and_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `carry_add[i] = (b_i + c_i + carry_add[i - 1] - a_i) / 2^N_BITS_PER_BYTE` with
  `carry_add[-1] = 0`.
- `carry_sub[i] = (a_i + c_i - b_i + carry_sub[i - 1]) / 2^N_BITS_PER_BYTE` with
  `carry_sub[-1] = 0`.
- `is_bitwise = opcode_xor_flag + opcode_or_flag + opcode_and_flag`
- `bitwise = opcode_xor_flag + 2 * opcode_or_flag + 3 * opcode_and_flag`.
- `rs_idx_diff = rs1_idx - rs2_idx`
- `rd_idx_diff_1 = rd_idx - rs1_idx`
- `rd_idx_diff_2 = rd_idx - rs2_idx`

### 1.3 Constraints

`enabler`, `opcode_*_flags` and `in_place_flags` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `in_place_flag_i * (1 - in_place_flag_i)`

if in-place flag is 1 then register diff (or one of register diffs) is 0

- `in_place_flag_1 * rs_idx_diff`
- `in_place_flag_2 * rd_idx_diff_1 * rd_idx_diff_2`

read instruction from the Program segment

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, rs2_idx)`

registers update

- `- enabler * RegsImm(pc, clk)`
- `+ enabler * RegsImm(pc + 4, clk + 1)`

read from rs1

- `- enabler * RegsRW(rs1_idx, rs1_prev_clk, b_0, b_1, b_2, b_3)`
- `+ enabler * RegsRW(rs1_idx, clk, b_0, b_1, b_2, b_3)`
- `- RC_20(clk - rs1_prev_clk - enabler)`

read from rs2

- `- enabler * RegsRW(rs2_idx, rs2_prev_clk, c_0, c_1, c_2, c_3)`
- `+ enabler * RegsRW(rs2_idx, clk, c_0, c_1, c_2, c_3)`
- `- (1 - in_place_flag_1) * RC_20(clk - rs2_prev_clk - enabler)`
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

- `- enabler * RegsRW(rd_idx, rd_prev_clk, rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3)`
- `+ enabler * RegsRW(rd_idx, clk, a_0, a_1, a_2, a_3)`
- `- (1 - in_place_flag_2) * RC_20(clk - rd_prev_clk - enabler)`
- `in_place_flag_2 * (clk - rd_prev_clk)`

## 2. Base ALU Imm (addi/subi/xori/ori/andi)

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
- `enabler = opcode_add_flag + opcode_sub_flag + opcode_xor_flag + opcode_or_flag + opcode_and_flag`.
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

`enabler`, `opcode_*_flags`, `in_place_flag` and `imm_msb` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
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

- `- enabler * RegsImm(pc, clk)`
- `+ enabler * RegsImm(pc + 4, clk + 1)`

read from rs1

- `- enabler * RegsRW(rs1_idx, rs1_prev_clk, b_0, b_1, b_2, b_3)`
- `+ enabler * RegsRW(rs1_idx, clk, b_0, b_1, b_2, b_3)`
- `- RC_20(clk - rs1_prev_clk - enabler)`

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

- `- enabler * RegsRW(rd_idx, rd_prev_clk, rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3)`
- `+ enabler * RegsRW(rd_idx, clk, a_0, a_1, a_2, a_3)`
- `- (1 - in_place_flag) * RC_20(clk - rd_prev_clk - enabler)`
- `in_place_flag * (clk - rd_prev_clk)`

## 3. Shifts Reg (sll/srl/sra)

### 3.1 Columns

- pc
- clk
- in_place_flag_1
- in_place_flag_2

- rd_idx
- rd_prev_clk
- rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3.
- a[0:3] — limbs of the value written to `rd`.

- rs1_idx
- rs1_prev_clk
- b[0:3] — limbs of `rs1` with b[3] containing just 7 bits
- b_sign

- rs2_idx
- rs2_prev_clk
- c[0:3] — limbs of `rs2`

- opcode_sll_flag
- opcode_srl_flag
- opcode_sra_flag

- bit_multiplier_left
- bit_multiplier_right
- bit_shift_marker[0:7]
- limb_shift_marker[0:3]
- bit_shift_carry[0:3]

### 3.2 Variables

- `enabler = opcode_sll_flag + opcode_srl_flag + opcode_sra_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`
- `left_shift = opcode_sll_flag`
- `right_shift = opcode_srl_flag + opcode_sra_flag`.
- `bit_multiplier = Σ 2^i * bit_shift_marker[i]`
- `bit_shift = Σ i * bit_shift_marker[i]`.
- `limb_shift = Σ i * limb_shift_marker[i]`.
- `shift_amount = limb_shift * 8 + bit_shift`.
- `b[3] = b[3] + 2^7 * b_sign`.
- `rs_idx_diff = rs1_idx - rs2_idx`
- `rd_idx_diff_1 = rd_idx - rs1_idx`
- `rd_idx_diff_2 = rd_idx - rs2_idx`

### 3.3 Constraints

`enabler`, `opcode_*_flags`, `b_sign` and `in_place_flag_i` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `in_place_flag_i * (1 - in_place_flag_i)`
- `b_sign * (1 - b_sign)`

if in-place flag is 1 then register diff (or one of register diffs) is 0

- `in_place_flag_1 * rs_idx_diff`
- `in_place_flag_2 * rd_idx_diff_1 * rd_idx_diff_2`

hot-one encodings must contain at most one activation

- `bit_shift_marker[i] * (1 - bit_shift_marker[i])`
- `limb_shift_marker[i] * (1 - limb_shift_marker[i])`
- `Σ bit_shift_marker[i] = enabler`
- `Σ limb_shift_marker[i] = enabler`

bit_multiplier are correctly formed

- `bit_multiplier_left - left_shift * bit_multiplier`
- `bit_multiplier_right - right_shift * bit_multiplier`

read instruction from the Program segment

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, rs2_idx)`

registers update

- `- enabler * RegsImm(pc, clk)`
- `+ enabler * RegsImm(pc + 4, clk + 1)`

read from rs1

- `- enabler * RegsRW(rs1_idx, rs1_prev_clk, b[0], b[1], b[2], b[3])`
- `+ enabler * RegsRW(rs1_idx, clk, b[0], b[1], b[2], b[3])`
- `- RC_20(clk - rs1_prev_clk - enabler)`

read from rs2

- `- enabler * RegsRW(rs2_idx, rs2_prev_clk, c[0], c[1], c[2], c[3])`
- `+ enabler * RegsRW(rs2_idx, clk, c[0], c[1], c[2], c[3])`
- `- (1 - in_place_flag_1) * RC_20(clk - rs2_prev_clk - enabler)`
- `in_place_flag_1 * (clk - rs2_prev_clk)`

the 5 first bits of c[0] shift `limb_shift` full limbs and `bit_shift` bits

- `- RC_20(2^3 - 1 - (c[0] - limb_shift * 8 - bit_shift) / 2^5)`

left shift constraints, for i in [0:3] and for j in [0:3]:

- `left_shift * limb_shift_marker[i] * a[j]` for `j < i`.
- `left_shift * limb_shift_marker[i] * (a[j] + 2^8 * bit_shift_carry[j - i]) - limb_shift_marker[i] * b[j - i] * bit_multiplier_left`
  for `j == i`
- `left_shift * limb_shift_marker[i] * (a[j] - (bit_shift_carry[j - i - 1] - 2^8 * bit_shift_carry[j - i])) - limb_shift_marker[i] * b[j - i] * bit_multiplier_left`
  for `j > i`.

right shift constraints, for i in [0:3] and for j in [0:3]:

- `right_shift * limb_shift_marker[i] * (a[j] - b_sign * (2^8 - 1))` for
  `j > 3 - i`.
- `b_sign * (bit_multiplier_right - 1) * 2^8 + right_shift * (b[j + i] - bit_shift_carry[j + i]) - a[j] * bit_multiplier_right`
  if `j == 3 - i`
- `bit_shift_carry[j + i + 1] * right_shift * 2^8 + right_shift * (b[j + i] - bit_shift_carry[j + i]) - a[j] * bit_multiplier_right`
  if `j < 3 - i`

shift carries should no exceed 2^bit_shift

- `- RC_8_8(bit_multiplier - enabler - bit_shift_carry[0], bit_multiplier - enabler - bit_shift_carry[1])`
- `- RC_8_8(bit_multiplier - enabler - bit_shift_carry[2], bit_multiplier - enabler - bit_shift_carry[3])`

range check a

- `- RC_8_8(a[0], a[1])`
- `- RC_8_8(a[2], a[3])`

write to rd

- `- enabler * RegsRW(rd_idx, rd_prev_clk, rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3)`
- `+ enabler * RegsRW(rd_idx, clk, a[0], a[1], a[2], a[3])`
- `- (1 - in_place_flag_2) * RC_20(clk - rd_prev_clk - enabler)`
- `in_place_flag_2 * (clk - rd_prev_clk)`

## 4. Shifts Imm (slli/srli/srai)

### 4.1 Columns

- pc
- clk
- in_place_flag

- rd_idx
- rd_prev_clk
- rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3.
- a[0:3] — limbs of the value written to `rd`.

- rs1_idx
- rs1_prev_clk
- b[0:3] — limbs of `rs1` with b[3] containing just 7 bits
- b_sign

- imm (imm[0:4])

- opcode_sll_flag
- opcode_srl_flag
- opcode_sra_flag

- bit_multiplier_left
- bit_multiplier_right
- bit_shift_marker[0:7]
- limb_shift_marker[0:3]
- bit_shift_carry[0:3]

### 4.2 Variables

- `enabler = opcode_sll_flag + opcode_srl_flag + opcode_sra_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`
- `left_shift = opcode_sll_flag`
- `right_shift = opcode_srl_flag + opcode_sra_flag`.
- `bit_multiplier = Σ 2^i * bit_shift_marker[i]`
- `bit_shift = Σ i * bit_shift_marker[i]`.
- `limb_shift = Σ i * limb_shift_marker[i]`.
- `shift_amount = limb_shift * 8 + bit_shift`.
- `b[3] = b[3] + 2^7 * b_sign`.
- `r_idx_diff = rd_idx - rs1_idx`

### 4.3 Constraints

`enabler`, `opcode_*_flags`, `b_sign` and `in_place_flag` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `in_place_flag * (1 - in_place_flag)`
- `b_sign * (1 - b_sign)`

if in-place flag is 1 then rd_idx == rs1_idx

- `in_place_flag * r_idx_diff`

hot-one encodings must contain at most one activation

- `bit_shift_marker[i] * (1 - bit_shift_marker[i])`
- `limb_shift_marker[i] * (1 - limb_shift_marker[i])`
- `Σ bit_shift_marker[i] = enabler`
- `Σ limb_shift_marker[i] = enabler`

bit_multiplier are correctly formed

- `bit_multiplier_left - left_shift * bit_multiplier`
- `bit_multiplier_right - right_shift * bit_multiplier`

read instruction from the Program segment

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, imm)`

registers update

- `- enabler * RegsImm(pc, clk)`
- `+ enabler * RegsImm(pc + 4, clk + 1)`

read from rs1

- `- enabler * RegsRW(rs1_idx, rs1_prev_clk, b[0], b[1], b[2], b[3])`
- `+ enabler * RegsRW(rs1_idx, clk, b[0], b[1], b[2], b[3])`
- `- RC_20(clk - rs1_prev_clk - enabler)`

the 5 first bits of imm shift `limb_shift` full limbs and `bit_shift` bits

- `- RC_20(2^3 - 1 - (imm - limb_shift * 8 - bit_shift) / 2^5)`

left shift constraints, for i in [0:3] and for j in [0:3]:

- `left_shift * limb_shift_marker[i] * a[j]` for `j < i`.
- `left_shift * limb_shift_marker[i] * (a[j] + 2^8 * bit_shift_carry[j - i]) - limb_shift_marker[i] * b[j - i] * bit_multiplier_left`
  for `j == i`
- `left_shift * limb_shift_marker[i] * (a[j] - (bit_shift_carry[j - i - 1] - 2^8 * bit_shift_carry[j - i])) - limb_shift_marker[i] * b[j - i] * bit_multiplier_left`
  for `j > i`.

right shift constraints, for i in [0:3] and for j in [0:3]:

- `right_shift * limb_shift_marker[i] * (a[j] - b_sign * (2^8 - 1))` for
  `j > 3 - i`.
- `b_sign * (bit_multiplier_right - 1) * 2^8 + right_shift * (b[j + i] - bit_shift_carry[j + i]) - a[j] * bit_multiplier_right`
  if `j == 3 - i`
- `bit_shift_carry[j + i + 1] * right_shift * 2^8 + right_shift * (b[j + i] - bit_shift_carry[j + i]) - a[j] * bit_multiplier_right`
  if `j < 3 - i`

shift carries should no exceed 2^bit_shift

- `- RC_8_8(bit_multiplier - enabler - bit_shift_carry[0], bit_multiplier - enabler - bit_shift_carry[1])`
- `- RC_8_8(bit_multiplier - enabler - bit_shift_carry[2], bit_multiplier - enabler - bit_shift_carry[3])`

range check a

- `- RC_8_8(a[0], a[1])`
- `- RC_8_8(a[2], a[3])`

write to rd

- `- enabler * RegsRW(rd_idx, rd_prev_clk, rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3)`
- `+ enabler * RegsRW(rd_idx, clk, a[0], a[1], a[2], a[3])`
- `- (1 - in_place_flag) * RC_20(clk - rd_prev_clk - enabler)`
- `in_place_flag * (clk - rd_prev_clk)`
