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

- opcode_add_flag
- opcode_sub_flag
- opcode_xor_flag
- opcode_or_flag
- opcode_and_flag

### 2.2 Variables

- `enabler = opcode_add_flag + opcode_sub_flag + opcode_xor_flag + opcode_or_flag + opcode_and_flag`.
- `is_bitwise = opcode_xor_flag + opcode_or_flag + opcode_and_flag`
- `bitwise = opcode_xor_flag + 2 * opcode_or_flag + 3 * opcode_and_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `imm = imm_0 + imm_1 * 2^8 + imm_msb * 2^11`
- `sext_imm_0 = imm_0`
- `sext_imm_1 = imm_1 + 2^3 * (2^5 - 1) * imm_msb`
- `sext_imm_2 = (2^8 - 1) * imm_msb`
- `sext_imm_3 = (2^8 - 1) * imm_msb`
- `carry_add[i] = (b_i + sext_imm_i + carry_add[i - 1] - a_i) / 2^N_BITS_PER_BYTE`
  with `carry_add[-1] = 0`
- `carry_sub[i] = (a_i + sext_imm_i - b_i + carry_sub[i - 1]) / 2^N_BITS_PER_BYTE`
  with `carry_sub[-1] = 0`
- `r_idx_diff = rd_idx - rs1_idx`

### 2.3 Constraints

`enabler`, `opcode_*_flags`, `in_place_flag` and `imm_msb` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `in_place_flag * (1 - in_place_flag)`

read instruction from the Program segment

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, imm)`

range check imm (range checks sext_imm too)

- `- RC_8_3(imm_0, imm_1)`
- `imm_msb * (1 - imm_msb)`

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

## 5. Less Than Reg (slt/sltu)

### 5.1 Columns

- pc
- clk
- in_place_flag_1
- in_place_flag_2

- rd_idx
- rd_prev_clk
- rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3.
- cmp_result

- rs1_idx
- rs1_prev_clk
- b_0, b_1, b_2, b_3 — limbs of `rs1`.

- rs2_idx
- rs2_prev_clk
- c_0, c_1, c_2, c_3 — limbs of `rs2`

- opcode_slt_flag
- opcode_sltu_flag

- b_msb_f
- c_msb_f
- diff_marker[0:3]
- diff_val

### 5.2 Variables

- `enabler = opcode_slt_flag + opcode_sltu_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `rs_idx_diff = rs1_idx - rs2_idx`
- `rd_idx_diff_1 = rd_idx - rs1_idx`
- `rd_idx_diff_2 = rd_idx - rs2_idx`
- `b_diff = b_3 - b_msb_f`
- `c_diff = c_3 - c_msb_f`

### 5.3 Constraints

`enabler`, `opcode_*_flags` and `in_place_flags` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `in_place_flag_i * (1 - in_place_flag_i)`
- `diff_marker_i * (1 - diff_marker_i)`

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

msb field elements must match actual msb bytes

- `b_diff * (2^N_BITS_PER_BYTE - b_diff)`
- `c_diff * (2^N_BITS_PER_BYTE - c_diff)`

comparison logic for each limb i (from 3 down to 0), `prefix_sum` is the running
sum of `diff_marker_i` and
`diff = (if i == 3 then c_msb_f - b_msb_f else c_i - b_i) * (2 * a_0 - 1)`

- `(1 - prefix_sum) * diff`
- `diff_marker_i * (diff_val - diff)`

`prefix_sum` contains at most one activation (`prefix_sum = Σ diff_marker_i`)

- `prefix_sum * (1 - prefix_sum)`

if equal, result is 0

- `(1 - prefix_sum) * cmp_result`

range check msb field elements with sign consideration

- `- RC_8_8(b_msb_f + opcode_slt_flag * 2^(N_BITS_PER_BYTE-1), c_msb_f + opcode_slt_flag * 2^(N_BITS_PER_BYTE-1))`

range check diff_val is non-zero when prefix_sum = 1

- `- prefix_sum * RC_8_8(diff_val - 1, 0)`

result is boolean

- `cmp_result * (1 - cmp_result)`

write to rd

- `- enabler * RegsRW(rd_idx, rd_prev_clk, rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3)`
- `+ enabler * RegsRW(rd_idx, clk, cmp_result, 0, 0, 0)`
- `- (1 - in_place_flag_2) * RC_20(clk - rd_prev_clk - enabler)`
- `in_place_flag_2 * (clk - rd_prev_clk)`

## 6. Less Than Imm (slti/sltiu)

### 6.1 Columns

- pc
- clk
- in_place_flag

- rd_idx
- rd_prev_clk
- rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3.
- cmp_result

- rs1_idx
- rs1_prev_clk
- b_0, b_1, b_2, b_3 — limbs of `rs1`.

- imm_0 (imm[0:7])
- imm_1 (imm[8:10])
- imm_msb (imm[11])

- opcode_slti_flag
- opcode_sltiu_flag

- b_msb_f
- sext_imm_msb_f
- diff_marker[0:3]
- diff_val

### 6.2 Variables

- `enabler = opcode_slti_flag + opcode_sltiu_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `imm = imm_0 + imm_1 * 2^8 + imm_msb * 2^11`
- `sext_imm_0 = imm_0`
- `sext_imm_1 = imm_1 + 2^3 * (2^5 - 1) * imm_msb`
- `sext_imm_2 = (2^8 - 1) * imm_msb`
- `sext_imm_3 = (2^8 - 1) * imm_msb`
- `r_idx_diff = rd_idx - rs1_idx`
- `b_diff = b_3 - b_msb_f`
- `sext_imm_diff = sext_imm_3 - sext_imm_msb_f`

### 6.3 Constraints

`enabler`, `opcode_*_flags`, `in_place_flag` and `imm_msb` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `in_place_flag * (1 - in_place_flag)`

read instruction from the Program segment

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, imm)`

range check imm

- `- RC_8_3(imm_0, imm_1)`
- `imm_msb * (1 - imm_msb)`

registers update

- `- enabler * RegsImm(pc, clk)`
- `+ enabler * RegsImm(pc + 4, clk + 1)`

read from rs1

- `- enabler * RegsRW(rs1_idx, rs1_prev_clk, b_0, b_1, b_2, b_3)`
- `+ enabler * RegsRW(rs1_idx, clk, b_0, b_1, b_2, b_3)`
- `- RC_20(clk - rs1_prev_clk - enabler)`

msb field elements must match actual msb bytes

- `b_diff * (2^N_BITS_PER_BYTE - b_diff)`
- `sext_imm_diff * (2^N_BITS_PER_BYTE - sext_imm_diff)`

diff markers are boolean and sum correctly

- `diff_marker_i * (1 - diff_marker_i)`
- `prefix_sum * (1 - prefix_sum)`

comparison logic for each limb i (from 3 down to 0), `prefix_sum` is the running
sum of `diff_marker_i` and
`diff = (if i == 3 then sext_imm_msb_f - b_msb_f else sext_imm_i - b_i) * (2 * cmp_result - 1)`

- `(1 - prefix_sum) * diff`
- `diff_marker_i * (diff_val - diff)`

`prefix_sum` contains at most one activation (`prefix_sum = Σ diff_marker_i`)

- `prefix_sum * (1 - prefix_sum)`

if equal, result is 0

- `(1 - prefix_sum) * cmp_result`

range check msb field elements with sign consideration

- `- RC_8_8(b_msb_f + opcode_slti_flag * 2^(N_BITS_PER_BYTE-1), sext_imm_msb_f + opcode_slti_flag * 2^(N_BITS_PER_BYTE-1))`

range check diff_val is non-zero when prefix_sum = 1

- `- prefix_sum * RC_8_8(diff_val - 1, 0)`

result is boolean

- `cmp_result * (1 - cmp_result)`

if in-place flag is 1 then rd_idx == rs1_idx

- `in_place_flag * r_idx_diff`

write to rd

- `- enabler * RegsRW(rd_idx, rd_prev_clk, rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3)`
- `+ enabler * RegsRW(rd_idx, clk, cmp_result, 0, 0, 0)`
- `- (1 - in_place_flag) * RC_20(clk - rd_prev_clk - enabler)`
- `in_place_flag * (clk - rd_prev_clk)`

## 7. Branch Equal (beq/bne)

### 7.1 Columns

- pc
- clk
- in_place_flag

- rs1_idx
- rs1_prev_clk
- a_0, a_1, a_2, a_3 — limbs of `rs1`.

- rs2_idx
- rs2_prev_clk
- b_0, b_1, b_2, b_3 — limbs of `rs2`

- imm - equals M31(imm) if imm>=0 and - M31(imm) if imm<0

- cmp_result - jump branch if cmp_result is 1
- diff_inv_marker[0:3] - 0 everywhere but for i where `a[i] != b[i]` if such i
  exists, `diff_inv_marker[i] = (a[i] - b[i])^-1`
- branch_target

- opcode_beq_flag
- opcode_bne_flag

### 7.2 Variables

- `enabler = opcode_beq_flag + opcode_bne_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `rs_idx_diff = rs1_idx - rs2_idx`
- `cmp_eq = cmp_result * opcode_beq_flag + (1 - cmp_result) * opcode_bne_flag`
- `diff_inv_sum = cmp_eq + Σ (a_i - b_i) * diff_inv_marker[i]`

### 7.3 Constraints

`enabler`, `opcode_*_flags`, `in_place_flag` and `cmp_result` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `in_place_flag * (1 - in_place_flag)`
- `cmp_result * (1 - cmp_result)`

if in-place flag is 1 then rs1_idx == rs2_idx

- `in_place_flag * rs_idx_diff`

read instruction from the Program segment

- `- enabler * Program(pc, expected_opcode_id, rs1_idx, rs2_idx, imm)`

check branch target

- `branch_target - ( pc + imm * cmp_result + 4 * (1 - cmp_result) )`

registers update (conditional branch)

- `- enabler * RegsImm(pc, clk)`
- `+ enabler * RegsImm(branch_target, clk + 1))`

read from rs1

- `- enabler * RegsRW(rs1_idx, rs1_prev_clk, a_0, a_1, a_2, a_3)`
- `+ enabler * RegsRW(rs1_idx, clk, a_0, a_1, a_2, a_3)`
- `- RC_20(clk - rs1_prev_clk - enabler)`

read from rs2

- `- enabler * RegsRW(rs2_idx, rs2_prev_clk, b_0, b_1, b_2, b_3)`
- `+ enabler * RegsRW(rs2_idx, clk, b_0, b_1, b_2, b_3)`
- `- (1 - in_place_flag) * RC_20(clk - rs2_prev_clk - enabler)`
- `in_place_flag * (clk - rs2_prev_clk)`

check `cmp_eq`

- for i in [0:3]: `cmp_eq * ( a[i] - b[i] )`
- `enabler * (1 - diff_inv_sum)`

## 8. Branch Less Than (blt/bltu/bge/bgeu)

### 8.1 Columns

- pc
- clk
- in_place_flag

- rs1_idx
- rs1_prev_clk
- a_0, a_1, a_2, a_3 — limbs of `rs1`.

- rs2_idx
- rs2_prev_clk
- b_0, b_1, b_2, b_3 — limbs of `rs2`

- imm_0 (imm[0:7])
- imm_1 (imm[8:11])
- imm_msb (imm[12])
- branch_offset
- branch_offset_neg
- cmp_result
- cmp_lt

- opcode_blt_flag
- opcode_bltu_flag
- opcode_bge_flag
- opcode_bgeu_flag

- a_msb_f
- b_msb_f
- diff_marker_0, diff_marker_1, diff_marker_2, diff_marker_3
- diff_val

### 8.2 Variables

- `enabler = opcode_blt_flag + opcode_bltu_flag + opcode_bge_flag + opcode_bgeu_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `imm = imm_0 + imm_1 * 2^8 + imm_msb * 2^12`
- `rs_idx_diff = rs1_idx - rs2_idx`
- `lt = opcode_blt_flag + opcode_bltu_flag`
- `ge = opcode_bge_flag + opcode_bgeu_flag`
- `signed = opcode_blt_flag + opcode_bge_flag`
- `prefix_sum = Σ diff_marker_i`
- `a_diff = a_3 - a_msb_f`
- `b_diff = b_3 - b_msb_f`
- `branch_target = pc + branch_offset - branch_offset_neg`

### 8.3 Constraints

`enabler`, `opcode_*_flags`, `in_place_flag`, `imm_msb`, `cmp_result` and
`cmp_lt` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `in_place_flag * (1 - in_place_flag)`
- `imm_msb * (1 - imm_msb)`
- `cmp_result * (1 - cmp_result)`
- `cmp_lt * (1 - cmp_lt)`

if in-place flag is 1 then rs1_idx == rs2_idx

- `in_place_flag * rs_idx_diff`

comparison result consistency

- `cmp_lt - cmp_result * lt - (1 - cmp_result) * ge`

read instruction from the Program segment

- `- enabler * Program(pc, expected_opcode_id, rs1_idx, rs2_idx, imm)`

range check imm

- `- RC_8_4(imm_0, imm_1)`

branch offset calculation (sign-extended 13-bit offset)

- `(1 - imm_msb) * (branch_offset - 2 * imm)`
- `(1 - imm_msb) * branch_offset_neg`
- `imm_msb * (branch_offset + branch_offset_neg - (2^14 - 2 * imm))`

registers update (conditional branch)

- `- enabler * RegsImm(pc, clk)`
- `cmp_result * (+ enabler * RegsImm(branch_target, clk + 1))`
- `(1 - cmp_result) * (+ enabler * RegsImm(pc + 4, clk + 1))`

read from rs1

- `- enabler * RegsRW(rs1_idx, rs1_prev_clk, a_0, a_1, a_2, a_3)`
- `+ enabler * RegsRW(rs1_idx, clk, a_0, a_1, a_2, a_3)`
- `- RC_20(clk - rs1_prev_clk - enabler)`

read from rs2

- `- enabler * RegsRW(rs2_idx, rs2_prev_clk, b_0, b_1, b_2, b_3)`
- `+ enabler * RegsRW(rs2_idx, clk, b_0, b_1, b_2, b_3)`
- `- (1 - in_place_flag) * RC_20(clk - rs2_prev_clk - enabler)`
- `in_place_flag * (clk - rs2_prev_clk)`

msb field elements must match actual msb bytes

- `a_diff * (2^N_BITS_PER_BYTE - a_diff)`
- `b_diff * (2^N_BITS_PER_BYTE - b_diff)`

diff markers are boolean and sum correctly

- `diff_marker_i * (1 - diff_marker_i)`
- `prefix_sum * (1 - prefix_sum)`

comparison logic for each limb i (from 3 down to 0)

- `(1 - prefix_sum) * (if i == 3 then b_msb_f - a_msb_f else b_i - a_i) * (2 * cmp_lt - 1)`
- `diff_marker_i * (diff_val - (if i == 3 then b_msb_f - a_msb_f else b_i - a_i) * (2 * cmp_lt - 1))`

if equal, cmp_lt is 0

- `(1 - prefix_sum) * cmp_lt`

range check msb field elements with sign consideration

- `- Bitwise(a_msb_f + signed * 2^(N_BITS_PER_BYTE-1), b_msb_f + signed * 2^(N_BITS_PER_BYTE-1), 0, 0)`

range check diff_val is non-zero when prefix_sum = 1

- `- prefix_sum * Bitwise(diff_val - 1, 0, 0, 0)`

range check branch offset

- `- RC_8_8(branch_offset_neg, branch_offset)`

## 9. JAL/LUI

### 9.1 Columns

- pc
- clk

- rd_idx
- rd_prev_clk
- rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3.
- rd[0:3] — limbs of the value written to `rd`.
- rd_3_intermediate

- imm - equals M31(imm) if imm>=0 and - M31(imm) if imm<0

- branch_target

- opcode_jal_flag
- opcode_lui_flag

### 9.2 Variables

- `enabler = opcode_jal_flag + opcode_lui_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.

### 9.3 Constraints

`enabler` and `opcode_*_flags` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`

read instruction from the Program segment

- `- enabler * Program(pc, expected_opcode_id, rd_idx, imm, 0)`

check branch target

- `branch_target - (pc + opcode_jal_flag * imm + opcode_lui_flag * 4)`

registers update

- `- enabler * RegsImm(pc, clk)`
- `- enabler * RegsImm(branch_target, clk + 1)`

rd us correctly built from imm:

- `opcode_lui_flag * rd[0]`
- `opcode_lui_flag * (2^4 * imm - (rd[1] + rd[2] * 2**8 + rd[3] * 2**16))`
- `opcode_jal_flag * (pc + 4 - (rd[0] + rd[1] * 2**8 + rd[2] * 2**16 + rd[3] * 2**24))`

rd_3_intermediate selects if rd[3] needs to be RC6 or RC8

- `rd_3_intermediate - ( opcode_jal_flag * (2^6 - rd[3]) + opcode_lui_flag * rd[3] )`

range check rd:

- `- RC_8_8(rd[0], rd[1])`
- `- RC_8_8(rd[2], rd_3_intermediate)`

write to rd

- `- enabler * RegsRW(rd_idx, rd_prev_clk, rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3)`
- `+ enabler * RegsRW(rd_idx, clk, rd[0], rd[1], rd[2], rd[3])`
- `- RC_20(clk - rd_prev_clk - enabler)`

## 10. JALR

### 10.1 Columns

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
- imm_sign

- opcode_jalr_flag

- jump_target_0, jump_target_1, jump_target_2, jump_target_3
- carry_0, carry_1, carry_2, carry_3

### 10.2 Variables

- `enabler = opcode_jalr_flag`.
- `expected_opcode_id = opcode_jalr_flag * opcode_id_jalr`.
- `imm = imm_0 + imm_1 * 2^8 + imm_msb * 2^11`
- `r_idx_diff = rd_idx - rs1_idx`
- `sext_imm_0 = imm_0`
- `sext_imm_1 = imm_1 + imm_msb * 2^3`
- `sext_imm_2 = imm_sign * (2^N_BITS_PER_BYTE - 1)`
- `sext_imm_3 = imm_sign * (2^N_BITS_PER_BYTE - 1)`

### 10.3 Constraints

`enabler`, `in_place_flag`, `imm_msb`, `imm_sign` and `opcode_jalr_flag` are
booleans

- `enabler * (1 - enabler)`
- `in_place_flag * (1 - in_place_flag)`
- `imm_msb * (1 - imm_msb)`
- `imm_sign * (1 - imm_sign)`
- `opcode_jalr_flag * (1 - opcode_jalr_flag)`

if in-place flag is 1 then rd_idx == rs1_idx

- `in_place_flag * r_idx_diff`

read instruction from the Program segment

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, imm)`

range check imm

- `- RC_8_3(imm_0, imm_1)`

sign extension consistency

- `(1 - imm_msb) * imm_sign`

registers update (jump to computed address)

- `- enabler * RegsImm(pc, clk)`
- `+ enabler * RegsImm(jump_target_0 + jump_target_1 * 2^8 + jump_target_2 * 2^16 + jump_target_3 * 2^24, clk + 1)`

read from rs1

- `- enabler * RegsRW(rs1_idx, rs1_prev_clk, b_0, b_1, b_2, b_3)`
- `+ enabler * RegsRW(rs1_idx, clk, b_0, b_1, b_2, b_3)`
- `- RC_20(clk - rs1_prev_clk - enabler)`

compute jump target (rs1 + sext_imm, set LSB to 0)

- `carry_0 * (1 - carry_0)`
- `carry_1 * (1 - carry_1)`
- `carry_2 * (1 - carry_2)`
- `carry_3 * (1 - carry_3)`
- `b_0 + sext_imm_0 + carry_0 * 2^N_BITS_PER_BYTE - jump_target_0 - (jump_target_0 % 2) * 2^N_BITS_PER_BYTE`
- `b_1 + sext_imm_1 + carry_1 * 2^N_BITS_PER_BYTE - jump_target_1 - carry_0`
- `b_2 + sext_imm_2 + carry_2 * 2^N_BITS_PER_BYTE - jump_target_2 - carry_1`
- `b_3 + sext_imm_3 + carry_3 * 2^N_BITS_PER_BYTE - jump_target_3 - carry_2`

store return address (pc + 4)

- `a_0 + a_1 * 2^8 + a_2 * 2^16 + a_3 * 2^24 - (pc + 4)`

range check a and jump_target

- `- RC_8_8(a_0, a_1)`
- `- RC_8_8(a_2, a_3)`
- `- RC_8_8(jump_target_0, jump_target_1)`
- `- RC_8_8(jump_target_2, jump_target_3)`

write to rd

- `- enabler * RegsRW(rd_idx, rd_prev_clk, rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3)`
- `+ enabler * RegsRW(rd_idx, clk, a_0, a_1, a_2, a_3)`
- `- (1 - in_place_flag) * RC_20(clk - rd_prev_clk - enabler)`
- `in_place_flag * (clk - rd_prev_clk)`

## 11. AUIPC

### 11.1 Columns

- pc
- clk

- rd_idx
- rd_prev_clk
- rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3.
- a_0, a_1, a_2, a_3 — limbs of the value written to `rd`.

- imm_0 (imm[0:7])
- imm_1 (imm[8:15])
- imm_2 (imm[16:19])

- opcode_auipc_flag

- pc_decomp_1, pc_decomp_2
- carry_1, carry_2, carry_3

### 11.2 Variables

- `enabler = opcode_auipc_flag`.
- `expected_opcode_id = opcode_auipc_flag * opcode_id_auipc`.
- `imm = imm_1 * 2^8 + imm_2 * 2^16` (imm_0 is always 0 for AUIPC)
- `pc_decomp_0 = a_0` (LSB of pc equals LSB of result)
- `pc_decomp_3 = (pc - pc_decomp_0 - pc_decomp_1 * 2^8 - pc_decomp_2 * 2^16) / 2^24`
  (MSB of pc)

### 11.3 Constraints

`enabler` and `opcode_auipc_flag` are booleans

- `enabler * (1 - enabler)`
- `opcode_auipc_flag * (1 - opcode_auipc_flag)`

read instruction from the Program segment

- `- enabler * Program(pc, expected_opcode_id, rd_idx, 0, imm_0 + imm_1 * 2^8 + imm_2 * 2^16)`

range check imm

- `- RC_8_8(imm_0, imm_1)`
- `- RC_4_0(imm_2, 0)`

AUIPC: constrain imm_0 = 0 (12-bit left shift means LSB limb is 0)

- `imm_0`

registers update

- `- enabler * RegsImm(pc, clk)`
- `+ enabler * RegsImm(pc + 4, clk + 1)`

PC decomposition

- `pc - pc_decomp_0 - pc_decomp_1 * 2^8 - pc_decomp_2 * 2^16 - pc_decomp_3 * 2^24`

compute rd = pc + (imm << 12)

- `carry_1 * (1 - carry_1)`
- `carry_2 * (1 - carry_2)`
- `carry_3 * (1 - carry_3)`
- `pc_decomp_0 - a_0` (LSB unchanged)
- `pc_decomp_1 + imm_1 + carry_1 * 2^N_BITS_PER_BYTE - a_1 - carry_0 * 2^N_BITS_PER_BYTE`
- `pc_decomp_2 + imm_2 + carry_2 * 2^N_BITS_PER_BYTE - a_2 - carry_1 * 2^N_BITS_PER_BYTE`
- `pc_decomp_3 + carry_3 * 2^N_BITS_PER_BYTE - a_3 - carry_2 * 2^N_BITS_PER_BYTE`

range check pc decomposition and result

- `- RC_8_8(pc_decomp_1, pc_decomp_2)`
- `- RC_8_8(a_0, a_1)`
- `- RC_8_8(a_2, a_3)`

write to rd

- `- enabler * RegsRW(rd_idx, rd_prev_clk, rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3)`
- `+ enabler * RegsRW(rd_idx, clk, a_0, a_1, a_2, a_3)`
- `- RC_20(clk - rd_prev_clk - enabler)`

## 12. MUL

### 12.1 Columns

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

- opcode_mul_flag

- carry_0, carry_1, carry_2, carry_3

### 12.2 Variables

- `enabler = opcode_mul_flag`.
- `expected_opcode_id = opcode_mul_flag * opcode_id_mul`.
- `rs_idx_diff = rs1_idx - rs2_idx`
- `rd_idx_diff_1 = rd_idx - rs1_idx`
- `rd_idx_diff_2 = rd_idx - rs2_idx`
- `product_sum[i] = Σ_{k=0}^{i} b_k * c_{i-k}` (convolution sum for limb i)

### 12.3 Constraints

`enabler`, `opcode_mul_flag` and `in_place_flags` are booleans

- `enabler * (1 - enabler)`
- `opcode_mul_flag * (1 - opcode_mul_flag)`
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

multiplication with carries (schoolbook multiplication)

- `carry_i * (2^N_BITS_PER_BYTE - 1 - carry_i)` for i in [0,3]
- `product_sum[0] + carry_0 * 2^N_BITS_PER_BYTE - a_0 - carry_0` where
  `product_sum[0] = b_0 * c_0`
- `product_sum[1] + carry_1 * 2^N_BITS_PER_BYTE - a_1 - carry_0` where
  `product_sum[1] = b_0 * c_1 + b_1 * c_0`
- `product_sum[2] + carry_2 * 2^N_BITS_PER_BYTE - a_2 - carry_1` where
  `product_sum[2] = b_0 * c_2 + b_1 * c_1 + b_2 * c_0`
- `product_sum[3] + carry_3 * 2^N_BITS_PER_BYTE - a_3 - carry_2` where
  `product_sum[3] = b_0 * c_3 + b_1 * c_2 + b_2 * c_1 + b_3 * c_0`

range check carries and result

- `- RC_8_8(carry_0, carry_1)`
- `- RC_8_8(carry_2, carry_3)`
- `- RC_8_8(a_0, a_1)`
- `- RC_8_8(a_2, a_3)`

write to rd

- `- enabler * RegsRW(rd_idx, rd_prev_clk, rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3)`
- `+ enabler * RegsRW(rd_idx, clk, a_0, a_1, a_2, a_3)`
- `- (1 - in_place_flag_2) * RC_20(clk - rd_prev_clk - enabler)`
- `in_place_flag_2 * (clk - rd_prev_clk)`

## 13. MULH (mulh/mulhsu/mulhu)

### 13.1 Columns

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

- opcode_mulh_flag
- opcode_mulhsu_flag
- opcode_mulhu_flag

- a_mul_0, a_mul_1, a_mul_2, a_mul_3 — lower 32 bits of b \* c
- b_ext, c_ext — sign extension values
- carry_mul_0, carry_mul_1, carry_mul_2, carry_mul_3 — carries for lower
  multiplication
- carry_0, carry_1, carry_2, carry_3 — carries for upper multiplication

### 13.2 Variables

- `enabler = opcode_mulh_flag + opcode_mulhsu_flag + opcode_mulhu_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `rs_idx_diff = rs1_idx - rs2_idx`
- `rd_idx_diff_1 = rd_idx - rs1_idx`
- `rd_idx_diff_2 = rd_idx - rs2_idx`
- `signed = opcode_mulh_flag + opcode_mulhsu_flag` (both operands signed for
  MULH, rs1 only for MULHSU)
- `b_sign = b_ext / (2^N_BITS_PER_BYTE - 1)` (sign bit of rs1)
- `c_sign = c_ext / (2^N_BITS_PER_BYTE - 1)` (sign bit of rs2)

### 13.3 Constraints

`enabler`, `opcode_*_flags`, `in_place_flags`, `b_sign`, and `c_sign` are
booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `in_place_flag_i * (1 - in_place_flag_i)`
- `b_sign * (1 - b_sign)`
- `c_sign * (1 - c_sign)`

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

sign extension constraints

- `opcode_mulhu_flag * b_sign` (unsigned operands for MULHU)
- `(opcode_mulhu_flag + opcode_mulhsu_flag) * c_sign` (rs2 unsigned for MULHU
  and MULHSU)
- `b_ext - b_sign * (2^N_BITS_PER_BYTE - 1)`
- `c_ext - c_sign * (2^N_BITS_PER_BYTE - 1)`

sign bit extraction with XOR

- `- signed * Bitwise(b_3, 2^(N_BITS_PER_BYTE-1), b_3 + 2^(N_BITS_PER_BYTE-1) - 2 * b_sign * 2^(N_BITS_PER_BYTE-1), 3)`
- `- opcode_mulh_flag * Bitwise(c_3, 2^(N_BITS_PER_BYTE-1), c_3 + 2^(N_BITS_PER_BYTE-1) - 2 * c_sign * 2^(N_BITS_PER_BYTE-1), 3)`

lower 32-bit multiplication (b _ c = a_mul + 2^32 _ a)

- `carry_mul_i * (2^N_BITS_PER_BYTE - 1 - carry_mul_i)` for i in [0,3]
- `(Σ_{k=0}^{0} b_k * c_{0-k}) + carry_mul_0 * 2^N_BITS_PER_BYTE - a_mul_0 - carry_mul_0`
- `(Σ_{k=0}^{1} b_k * c_{1-k}) + carry_mul_1 * 2^N_BITS_PER_BYTE - a_mul_1 - carry_mul_0`
- `(Σ_{k=0}^{2} b_k * c_{2-k}) + carry_mul_2 * 2^N_BITS_PER_BYTE - a_mul_2 - carry_mul_1`
- `(Σ_{k=0}^{3} b_k * c_{3-k}) + carry_mul_3 * 2^N_BITS_PER_BYTE - a_mul_3 - carry_mul_2`

upper 32-bit multiplication (high part)

- `carry_i * (2^N_BITS_PER_BYTE - 1 - carry_i)` for i in [0,3]
- `carry_mul_3 + (Σ_{k=1}^{3} b_k * c_{3+0-k}) + 4 * (b_0 * c_ext + c_0 * b_ext) + carry_0 * 2^N_BITS_PER_BYTE - a_0 - carry_0`
- `carry_0 + (Σ_{k=2}^{3} b_k * c_{3+1-k}) + 3 * (b_1 * c_ext + c_1 * b_ext) + carry_1 * 2^N_BITS_PER_BYTE - a_1 - carry_0`
- `carry_1 + (Σ_{k=3}^{3} b_k * c_{3+2-k}) + 2 * (b_2 * c_ext + c_2 * b_ext) + carry_2 * 2^N_BITS_PER_BYTE - a_2 - carry_1`
- `carry_2 + 1 * (b_3 * c_ext + c_3 * b_ext) + carry_3 * 2^N_BITS_PER_BYTE - a_3 - carry_2`

range check carries and results

- `- RC_8_8(carry_mul_0, carry_mul_1)`
- `- RC_8_8(carry_mul_2, carry_mul_3)`
- `- RC_8_8(carry_0, carry_1)`
- `- RC_8_8(carry_2, carry_3)`
- `- RC_8_8(a_mul_0, a_mul_1)`
- `- RC_8_8(a_mul_2, a_mul_3)`
- `- RC_8_8(a_0, a_1)`
- `- RC_8_8(a_2, a_3)`

write to rd

- `- enabler * RegsRW(rd_idx, rd_prev_clk, rd_prev_val_0, rd_prev_val_1, rd_prev_val_2, rd_prev_val_3)`
- `+ enabler * RegsRW(rd_idx, clk, a_0, a_1, a_2, a_3)`
- `- (1 - in_place_flag_2) * RC_20(clk - rd_prev_clk - enabler)`
- `in_place_flag_2 * (clk - rd_prev_clk)`

## DIVREM (div/divu/rem/remu)

### Columns

- `b[4], c[4]`: limbs of dividend and divisor
- `q[4], r[4]`: limbs of quotient and remainder
- `zero_divisor`: flag for c = 0 case
- `r_zero`: flag for r = 0 case
- `b_sign, c_sign, q_sign, sign_xor`: sign tracking for signed operations
- `c_sum_inv, r_sum_inv`: multiplicative inverses for zero checks
- `r_prime[4], r_inv[4]`: absolute remainder and range check helpers
- `lt_marker[4], lt_diff`: comparison markers for |r| < |c|
- `opcode_div_flag, opcode_divu_flag, opcode_rem_flag, opcode_remu_flag`: opcode
  selectors

### Variables

- `rs1_data[4], rs2_data[4], rd_data[4]`: register data
- `rs1_in_place, rs2_in_place`: in-place flags for memory timing
- `opcode_id`: instruction opcode
- `rs1_idx, rs2_idx, rd_idx`: register indices

### Constraints

1. **Boolean constraints**: All flags are binary
   - `zero_divisor, r_zero, b_sign, c_sign, q_sign in {0, 1}`
   - `opcode_div_flag, opcode_divu_flag, opcode_rem_flag, opcode_remu_flag in {0, 1}`
   - `lt_marker[i] in {0, 1}` for i = 0..3

2. **Single opcode selection**: Exactly one opcode flag is set
   - `opcode_div_flag + opcode_divu_flag + opcode_rem_flag + opcode_remu_flag = 1`

3. **Division relation**: b = c \* q + r (mod 2^32)

   ```
   For i = 0..3:
     expected_limb[i] = carry[i-1] + sum(c[k] * q[i-k] for k=0..i) + r[i]
     carry[i] = (expected_limb[i] - b[i]) / 2^8
     Range check: (q[i], carry[i]) in RC_8_8
   ```

4. **Extended precision constraints**: Upper limbs consistency

   ```
   For j = 0..3:
     b_ext = b_sign * 255
     c_ext = c_sign * 255
     q_ext = q_sign * 255
     expected_ext[j] = carry_ext[j-1] + sum(c[k] * q[N+j-k] for k=j+1..3) +
                       sum(c[k] * q_ext + q[k] * c_ext for k=0..j) +
                       (1 - r_zero) * b_ext
     carry_ext[j] = (expected_ext[j] - b_ext) / 2^8
     Range check: (r[j], carry_ext[j]) in RC_8_8
   ```

5. **Zero divisor constraints**:
   - If `zero_divisor = 1`: `c[i] = 0` and `q[i] = 255` for all i
   - If `zero_divisor = 0` and valid: `c_sum * c_sum_inv = 1` where
     `c_sum = sum(c[i])`

6. **Zero remainder constraints**:
   - If `r_zero = 1`: `r[i] = 0` for all i
   - If `special_case = 0` and valid: `r_sum * r_sum_inv = 1` where
     `r_sum = sum(r[i])`
   - Special case constraint: `zero_divisor + r_zero <= 1`

7. **Sign constraints** (for signed operations):

   ```
   signed = opcode_div_flag + opcode_rem_flag
   If signed = 0: b_sign = c_sign = 0
   sign_xor = b_sign XOR c_sign = b_sign + c_sign - 2 * b_sign * c_sign
   ```

8. **Sign bit validation** (for signed operations):

   ```
   sign_mask = 128
   Bitwise check: (2 * (b[3] - b_sign * sign_mask), 2 * (c[3] - c_sign * sign_mask)) in Bitwise
   ```

9. **Quotient sign constraints**:

   ```
   q_sum = sum(q[i])
   If q_sum ≠ 0 and zero_divisor = 0: q_sign = sign_xor
   If q_sign ≠ sign_xor and zero_divisor = 0: q_sign = 0
   ```

10. **Absolute remainder calculation**:

    ```
    If sign_xor = 0: r_prime[i] = r[i] for all i
    If sign_xor = 1: Two's complement negation with carry propagation
      carry_lt[i] = (carry_lt[i-1] + r[i] + r_prime[i]) / 2^8
      carry_lt[i] * (carry_lt[i] - 1) = 0
      (r_prime[i] - 256) * r_inv[i] = 1
      If carry_lt[i] = 0: r_prime[i] = 0
    ```

11. **Magnitude comparison |r| < |c|**:

    ```
    For i = 3..0 (reverse order):
      diff[i] = r_prime[i] * (2 * c_sign - 1) + c[i] * (1 - 2 * c_sign)
      prefix_sum += lt_marker[i]
      If prefix_sum = 0: diff[i] = 0
      If lt_marker[i] = 1: lt_diff = diff[i]

    If not special_case: prefix_sum = 1
    If not special_case: Range check (lt_diff - 1) in RC_20
    ```

12. **Result selection**:

    ```
    is_div = opcode_div_flag + opcode_divu_flag
    rd_data[i] = is_div * q[i] + (1 - is_div) * r[i]
    ```

13. **Memory interactions**:

    ```
    RegsRW(rs1_idx, rs1_data, rs1_in_place)
    RegsRW(rs2_idx, rs2_data, rs2_in_place)
    RegsRW(rd_idx, rd_data, 0)
    ```

14. **Program interaction**:
    ```
    expected_opcode = opcode_div_flag * DIV + opcode_divu_flag * DIVU +
                      opcode_rem_flag * REM + opcode_remu_flag * REMU
    Program(pc, expected_opcode, rd_idx, rs1_idx, rs2_idx)
    ```

## Load Operations (loadw/loadb/loadh/loadbu/loadhu)

### Columns

- `read_data[4]`: raw bytes from memory
- `write_data[4]`: result limbs after load processing
- `shift_amount`: memory alignment offset (0-3)
- `sign_bit`: most significant bit for sign extension
- `opcode_loadw_flag, opcode_loadb_flag, opcode_loadbu_flag, opcode_loadh_flag, opcode_loadhu_flag`:
  opcode selectors

### Variables

- `rs1_data[4], rd_data[4]`: register data
- `rs1_in_place`: in-place flag for memory timing
- `opcode_id`: instruction opcode
- `rs1_idx, rd_idx`: register indices
- `imm_12`: sign-extended 12-bit immediate

### Constraints

1. **Boolean constraints**: All flags and bits are binary
   - `opcode_loadw_flag, opcode_loadb_flag, opcode_loadbu_flag, opcode_loadh_flag, opcode_loadhu_flag in {0, 1}`
   - `sign_bit in {0, 1}`

2. **Single opcode selection**: Exactly one opcode flag is set
   - `opcode_loadw_flag + opcode_loadb_flag + opcode_loadbu_flag + opcode_loadh_flag + opcode_loadhu_flag = 1`

3. **Sign bit extraction**: Extract MSB for sign extension

   ```
   For LOADB: sign_source = shift_amount
   For LOADH: sign_source = 1 + shift_amount
   For others: sign_source = 0

   sign_check = (opcode_loadb_flag + opcode_loadh_flag) * sign_bit
   Range check: (read_data[sign_source] - sign_bit * 128) in RC_20 (7-bit range)
   ```

4. **Load result construction**:

   ```
   For LOADW:
     write_data[i] = read_data[i] for all i

   For LOADB (signed):
     write_data[0] = read_data[shift_amount]
     write_data[i] = sign_bit * 255 for i > 0

   For LOADBU (unsigned):
     write_data[0] = read_data[shift_amount]
     write_data[i] = 0 for i > 0

   For LOADH (signed):
     write_data[i] = read_data[i + shift_amount] for i < 2
     write_data[i] = sign_bit * 255 for i >= 2

   For LOADHU (unsigned):
     write_data[i] = read_data[i + shift_amount] for i < 2
     write_data[i] = 0 for i >= 2
   ```

5. **Memory address calculation**:

   ```
   address = rs1_data + imm_12
   Memory load: (address, read_data)
   ```

6. **Shift amount validation**:

   ```
   For LOADB/LOADBU: shift_amount in {0, 1, 2, 3}
   For LOADH/LOADHU: shift_amount in {0, 2}
   For LOADW: shift_amount = 0
   ```

7. **Memory interactions**:

   ```
   RegsRW(rs1_idx, rs1_data, rs1_in_place)
   RegsRW(rd_idx, write_data, 0)
   Memory(address, read_data)
   ```

8. **Program interaction**:
   ```
   expected_opcode = opcode_loadw_flag * LOADW + opcode_loadb_flag * LOADB +
                     opcode_loadbu_flag * LOADBU + opcode_loadh_flag * LOADH +
                     opcode_loadhu_flag * LOADHU
   Program(pc, expected_opcode, rd_idx, rs1_idx, imm_12)
   ```

## Store Operations (storew/storeh/storeb)

### Columns

- `rs2_data[4]`: source register data to store
- `prev_data[4]`: existing memory content at target address
- `write_data[4]`: final memory content after store operation
- `shift_amount`: memory alignment offset (0-3)
- `opcode_storew_flag, opcode_storeh_flag, opcode_storeb_flag`: opcode selectors

### Variables

- `rs1_data[4], rs2_data[4]`: register data
- `rs1_in_place, rs2_in_place`: in-place flags for memory timing
- `opcode_id`: instruction opcode
- `rs1_idx, rs2_idx`: register indices
- `imm_12`: sign-extended 12-bit immediate

### Constraints

1. **Boolean constraints**: All flags are binary
   - `opcode_storew_flag, opcode_storeh_flag, opcode_storeb_flag in {0, 1}`

2. **Single opcode selection**: Exactly one opcode flag is set
   - `opcode_storew_flag + opcode_storeh_flag + opcode_storeb_flag = 1`

3. **Store result construction**: Merge source data with existing memory

   ```
   For STOREW:
     write_data[i] = rs2_data[i] for all i

   For STOREH:
     For i in range(shift_amount, shift_amount + 2):
       write_data[i] = rs2_data[i - shift_amount]
     For other i:
       write_data[i] = prev_data[i]

   For STOREB:
     write_data[shift_amount] = rs2_data[0]
     For other i:
       write_data[i] = prev_data[i]
   ```

4. **Memory address calculation**:

   ```
   address = rs1_data + imm_12
   Memory store: (address, write_data)
   ```

5. **Shift amount validation**:

   ```
   For STOREB: shift_amount in {0, 1, 2, 3}
   For STOREH: shift_amount in {0, 2}
   For STOREW: shift_amount = 0
   ```

6. **Memory interactions**:

   ```
   RegsRW(rs1_idx, rs1_data, rs1_in_place)
   RegsRW(rs2_idx, rs2_data, rs2_in_place)
   Memory(address, prev_data) -> Memory(address, write_data)
   ```

7. **Program interaction**:
   ```
   expected_opcode = opcode_storew_flag * STOREW + opcode_storeh_flag * STOREH +
                     opcode_storeb_flag * STOREB
   Program(pc, expected_opcode, rs1_idx, rs2_idx, imm_12)
   ```
