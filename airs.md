# AIRS

## 0. Design choices

### 0.1 PC is a M31

PC is a M31 since representing it as a u32 would add a lot of overhead (to do
simple pc update, +3 columns per opcodes) It could have been a u30 but that
would've required range checks for each pc op.

Also to assert a u32 is a M31, we use the `RC_M31(lsl, msl)` to check that the
least significant limb (`lsl`) and most significant limb (`msl`) are those of a
M31 (in LE, max M31 is `0b01111111_11111111_11111111_11111110`). The middle
limbs are simply 8_8 RCed.

### 0.2 Imm encoding

Two opcodes are encoded with the `U` format: LUI and AUIPC.

`imm_felt` -- AUIPC is `x[rd] = pc + sext(decoded_imm[31:12] << 12)`. Here rd
stores a pc-based address so `pc + imm` will be a M31. For that reason we chose
to represent imm directly as a
`M31(sign(decoded_imm) * (abs(decoded_imm) << 12))`. As so, this keeps AUIPC's
AIR minimal, relying on the following assumption: `abs(decoded_imm) << 12` does
not overflow M31.

`imm_decoded` -- LUI is `x[rd] = sext(decoded_imm[31:12] << 12)`. It should be
possible to load any u32 into rd. Since the AIR has to do the sign-extension, it
is necessary to have `decoded_imm` limbs (`decoded_imm[0..2]`,
`decoded_imm[3..10]`, and `decoded_imm_msb`).

`imm_truncated` -- SHIFTS (I-type) use the 5 first bits of the decoded immediate
for the shifting amount. The AIR expects the program to have
`decoded_immediate & (2^5 - 1)` as operand (should be taken care by the
transpiler).

### 0.3 Instructions ordering

- R-type: `(opcode_id, rd_idx, rs1_idx, rs2_idx)`
- S-type: `(opcode_id, rs1_idx, rs2_idx, imm_felt)`
- I-type: `(opcode_id, rd, rs1, imm_decoded)`
- I-type (shift): `(opcode_id, rd, rs1, imm_truncated)`
- I-type (load): `(opcode_id, rs1, rd, imm_felt)`
- U-type (lui): `(opcode_id, rd_idx, decoded_imm, 0)`
- U-type (auipc): `(opcode_id, rd_idx, imm_felt, 0)`
- J-type: `(opcode_id, rd, imm, 0)`

## 1. Base ALU Reg (add/sub/xor/or/and)

### 1.0 Factorization cost

Extra cost compared to having 2 components: add/sub - xor/or/and

- for bitwise: 4T
- for add/sub: 4T + 2L (`max_log_size = 21`) or 4T + 4L (`max_log_size = 20`)

=> 4 unused cells per bitwise and 8 to 12 cells per add/sub

### 1.1 Columns

- pc
- clk

- rd_idx
- rd_prev_clk
- rd_prev[0..3]
- rd[0..3]

- rs1_idx
- rs1_prev_clk
- rs1[0..3]

- rs2_idx
- rs2_prev_clk
- rs2[0..3]

- opcode_add_flag
- opcode_sub_flag
- opcode_xor_flag
- opcode_or_flag
- opcode_and_flag

### 1.2 Variables

- `enabler = opcode_add_flag + opcode_sub_flag + opcode_xor_flag + opcode_or_flag + opcode_and_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `carry_add[i+1] = (rs1[i+1] + rs2[i+1] + carry_add[i] - rd[i+1]) / 2^N_BITS_PER_BYTE`
  with `carry_add[0] = 0`.
- `carry_sub[i+1] = (rd[i+1] + rs2[i+1] - rs1[i+1] + carry_sub[i]) / 2^N_BITS_PER_BYTE`
  with `carry_sub[0] = 0`.
- `is_bitwise = opcode_xor_flag + opcode_or_flag + opcode_and_flag`.
- `bitwise_id = opcode_xor_flag + 2 * opcode_or_flag + 3 * opcode_and_flag + 4 * (opcode_add_flag + opcode_sub_flag)`.

### 1.3 Constraints

`enabler` and `opcode_*_flags` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`

read instruction from the Program segment (R-type)

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, rs2_idx)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_idx, rs1_prev_clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `+ enabler * Memory(REG_AS, rs1_idx, clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `- RC_20(clk - rs1_prev_clk)`

read from rs2

- `- enabler * Memory(REG_AS, rs2_prev_clk, rs2[0], rs2[1], rs2[2], rs2[3])`
- `+ enabler * Memory(REG_AS, clk, rs2[0], rs2[1], rs2[2], rs2[3])`
- `- RC_20(clk - rs2_prev_clk)`

check carries

- `opcode_add_flag * carry_add[i] * (1 - carry_add[i])`
- `opcode_sub_flag * carry_sub[i] * (1 - carry_sub[i])`

perform bitwise operation and RC rd (`rd[i]` is determined by `rs1[i]`,
`rs2[i]`, `carry[i-1]`, `carry[i]` i.e. 8 + 8 + 1 + 1 = 20 bits so `log_size` of
bitwise is 21 to avoid 4 extra columns of RC_8_8 for rd, test if worth it)

- `- is_bitwise * Bitwise(rs1[0], rs2[0], rd[0], bitwise_id)`
- `- is_bitwise * Bitwise(rs1[1], rs2[1], rd[1], bitwise_id)`
- `- is_bitwise * Bitwise(rs1[2], rs2[2], rd[2], bitwise_id)`
- `- is_bitwise * Bitwise(rs1[3], rs2[3], rd[3], bitwise_id)`

write to rd

- `- enabler * Memory(REG_AS, rd_idx, rd_prev_clk, rd_prev[0], rd_prev[1], rd_prev[2], rd_prev[3])`
- `+ enabler * Memory(REG_AS, rd_idx, clk, rd[0], rd[1], rd[2], rd[3])`
- `- RC_20(clk - rd_prev_clk)`

## 2. Base ALU Imm (addi/subi/xori/ori/andi)

### 2.0 Factorization cost

Same as 1.0

### 2.1 Columns

- pc
- clk

- rd_idx
- rd_prev_clk
- rd_prev[0..3]
- rd[0..3]

- rs1_idx
- rs1_prev_clk
- rs1[0..3]

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
- `is_bitwise = opcode_xor_flag + opcode_or_flag + opcode_and_flag`
- `bitwise_id = opcode_xor_flag + 2 * opcode_or_flag + 3 * opcode_and_flag`.

### 2.3 Constraints

`enabler` and `opcode_*_flags` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`

read instruction from the Program segment (I-type)

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, imm)`

range check imm (range checks sext_imm too)

- `- RC_8_3(imm_0, imm_1)`
- `imm_msb * (1 - imm_msb)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_idx, rs1_prev_clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `+ enabler * Memory(REG_AS, rs1_idx, clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `- RC_20(clk - rs1_prev_clk)`

check carries

- `opcode_add_flag * carry_add[i] * (1 - carry_add[i])`
- `opcode_sub_flag * carry_sub[i] * (1 - carry_sub[i])`

perform bitwise operation and RC rd (same tradeoff as 1.3)

- `- is_bitwise * Bitwise(rs1[0], sext_imm_0, rd[0], bitwise_id)`
- `- is_bitwise * Bitwise(rs1[1], sext_imm_1, rd[1], bitwise_id)`
- `- is_bitwise * Bitwise(rs1[2], sext_imm_2, rd[2], bitwise_id)`
- `- is_bitwise * Bitwise(rs1[3], sext_imm_3, rd[3], bitwise_id)`

range check a (redundant for bitwise)

- `- RC_8_8(rd[0], rd[1])`
- `- RC_8_8(rd[2], rd[3])`

write to rd

- `- enabler * Memory(REG_AS, rd_prev_clk, rd_prev[0], rd_prev[1], rd_prev[2], rd_prev[3])`
- `+ enabler * Memory(REG_AS, clk, rd[0], rd[1], rd[2], rd[3])`
- `- RC_20(clk - rd_prev_clk)`

## 3. Shifts Reg (sll/srl/sra)

### 3.1 Columns

- pc
- clk

- rd_idx
- rd_prev_clk
- rd_prev[0..3]
- rd[0..3]

- rs1_idx
- rs1_prev_clk
- rs1[0..3] - `rs[3]` has 7 bit
- rs1_sign

- rs2_idx
- rs2_prev_clk
- rs2[0..3]

- opcode_sll_flag
- opcode_srl_flag
- opcode_sra_flag

- bit_multiplier_left
- bit_multiplier_right
- bit_shift_marker[0..7]
- limb_shift_marker[0..3]
- bit_shift_carry[0..3]

### 3.2 Variables

- `enabler = opcode_sll_flag + opcode_srl_flag + opcode_sra_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`
- `left_shift = opcode_sll_flag`
- `right_shift = opcode_srl_flag + opcode_sra_flag`.
- `bit_multiplier = Σ 2^i * bit_shift_marker[i]`
- `bit_shift = Σ i * bit_shift_marker[i]`.
- `limb_shift = Σ i * limb_shift_marker[i]`.
- `shift_amount = limb_shift * 8 + bit_shift`.
- `rs1[3] = rs1[3] + 2^7 * rs1_sign`.

### 3.3 Constraints

`enabler`, `opcode_*_flags` and `b_sign` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `b_sign * (1 - b_sign)`

hot-one encodings must contain at most one activation

- `bit_shift_marker[i] * (1 - bit_shift_marker[i])`
- `limb_shift_marker[i] * (1 - limb_shift_marker[i])`
- `Σ bit_shift_marker[i] = enabler`
- `Σ limb_shift_marker[i] = enabler`

bit_multiplier are correctly formed

- `bit_multiplier_left - left_shift * bit_multiplier`
- `bit_multiplier_right - right_shift * bit_multiplier`

read instruction from the Program segment (R-type)

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, rs2_idx)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_idx, rs1_prev_clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `+ enabler * Memory(REG_AS, rs1_idx, clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `- RC_20(clk - rs1_prev_clk)`

read from rs2

- `- enabler * Memory(REG_AS, rs2_idx, rs2_prev_clk, rs2[0], rs2[1], rs2[2], rs2[3])`
- `+ enabler * Memory(REG_AS, rs2_idx, clk, rs2[0], rs2[1], rs2[2], rs2[3])`
- `- RC_20(clk - rs2_prev_clk)`

the 5 first bits of `rs2[0]` shift `limb_shift` full limbs and `bit_shift` bits

- `- RC_20(2^3 - 1 - (rs2[0] - shift_amount) / 2^5)`

left shift constraints, for i in [0..3] and for j in [0..3]:

- `left_shift * limb_shift_marker[i] * rd[j]` for `j < i`.
- `left_shift * limb_shift_marker[i] * (rd[j] + 2^8 * bit_shift_carry[j - i]) - limb_shift_marker[i] * rs1[j - i] * bit_multiplier_left`
  for `j == i`
- `left_shift * limb_shift_marker[i] * (rd[j] - (bit_shift_carry[j - i - 1] - 2^8 * bit_shift_carry[j - i])) - limb_shift_marker[i] * rs1[j - i] * bit_multiplier_left`
  for `j > i`.

right shift constraints, for i in [0..3] and for j in [0..3]:

- `right_shift * limb_shift_marker[i] * (rd[j] - rs1_sign * (2^8 - 1))` for
  `j > 3 - i`.
- `rs1_sign * (bit_multiplier_right - 1) * 2^8 + right_shift * (rs1[j + i] - bit_shift_carry[j + i]) - rd[j] * bit_multiplier_right`
  if `j == 3 - i`
- `bit_shift_carry[j + i + 1] * right_shift * 2^8 + right_shift * (rs1[j + i] - bit_shift_carry[j + i]) - rd[j] * bit_multiplier_right`
  if `j < 3 - i`

shift carries should no exceed 2^bit_shift

- `- RC_8_8(bit_multiplier - enabler - bit_shift_carry[0], bit_multiplier - enabler - bit_shift_carry[1])`
- `- RC_8_8(bit_multiplier - enabler - bit_shift_carry[2], bit_multiplier - enabler - bit_shift_carry[3])`

range check rd

- `- RC_8_8(rd[0], rd[1])`
- `- RC_8_8(rd[2], rd[3])`

write to rd

- `- enabler * Memory(REG_AS, rd_idx, rd_prev_clk, rd_prev[0], rd_prev[1], rd_prev[2], rd_prev[3])`
- `+ enabler * Memory(REG_AS, rd_idx, clk, rd[0], rd[1], rd[2], rd[3])`
- `- RC_20(clk - rd_prev_clk)`

## 4. Shifts Imm (slli/srli/srai)

### 4.1 Columns

- pc
- clk

- rd_idx
- rd_prev_clk
- rd_prev[0..3]
- rd[0..3]

- rs1_idx
- rs1_prev_clk
- rs1[0..3]
- rs1_sign

- imm_truncated (imm[0:4])

- opcode_sll_flag
- opcode_srl_flag
- opcode_sra_flag

- bit_multiplier_left
- bit_multiplier_right
- bit_shift_marker[0..7]
- limb_shift_marker[0..3]
- bit_shift_carry[0..3]

### 4.2 Variables

- `enabler = opcode_sll_flag + opcode_srl_flag + opcode_sra_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`
- `left_shift = opcode_sll_flag`
- `right_shift = opcode_srl_flag + opcode_sra_flag`.
- `bit_multiplier = Σ 2^i * bit_shift_marker[i]`
- `bit_shift = Σ i * bit_shift_marker[i]`.
- `limb_shift = Σ i * limb_shift_marker[i]`.
- `shift_amount = limb_shift * 8 + bit_shift`.
- `rs1[3] = rs1[3] + 2^7 * rs1_sign`.

### 4.3 Constraints

`enabler`, `opcode_*_flags` and `rs1_sign` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `b_sign * (1 - b_sign)`

hot-one encodings must contain at most one activation

- `bit_shift_marker[i] * (1 - bit_shift_marker[i])`
- `limb_shift_marker[i] * (1 - limb_shift_marker[i])`
- `Σ bit_shift_marker[i] = enabler`
- `Σ limb_shift_marker[i] = enabler`

bit_multiplier are correctly formed

- `bit_multiplier_left - left_shift * bit_multiplier`
- `bit_multiplier_right - right_shift * bit_multiplier`

read instruction from the Program segment (I-type)

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, imm_truncated)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_idx, rs1_prev_clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `+ enabler * Memory(REG_AS, rs1_idx, clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `- RC_20(clk - rs1_prev_clk)`

`imm_truncated` shift `limb_shift` full limbs and `bit_shift` bits

- `imm_truncated - shift_amount`

left shift constraints, for i in [0..3] and for j in [0..3]:

- `left_shift * limb_shift_marker[i] * rd[j]` for `j < i`.
- `left_shift * limb_shift_marker[i] * (rd[j] + 2^8 * bit_shift_carry[j - i]) - limb_shift_marker[i] * rs1[j - i] * bit_multiplier_left`
  for `j == i`
- `left_shift * limb_shift_marker[i] * (rd[j] - (bit_shift_carry[j - i - 1] - 2^8 * bit_shift_carry[j - i])) - limb_shift_marker[i] * rs1[j - i] * bit_multiplier_left`
  for `j > i`.

right shift constraints, for i in [0..3] and for j in [0..3]:

- `right_shift * limb_shift_marker[i] * (rd[j] - rs1_sign * (2^8 - 1))` for
  `j > 3 - i`.
- `rs1_sign * (bit_multiplier_right - 1) * 2^8 + right_shift * (rs1[j + i] - bit_shift_carry[j + i]) - rd[j] * bit_multiplier_right`
  if `j == 3 - i`
- `bit_shift_carry[j + i + 1] * right_shift * 2^8 + right_shift * (rs1[j + i] - bit_shift_carry[j + i]) - rd[j] * bit_multiplier_right`
  if `j < 3 - i`

shift carries should no exceed 2^bit_shift

- `- RC_8_8(bit_multiplier - enabler - bit_shift_carry[0], bit_multiplier - enabler - bit_shift_carry[1])`
- `- RC_8_8(bit_multiplier - enabler - bit_shift_carry[2], bit_multiplier - enabler - bit_shift_carry[3])`

range check rd

- `- RC_8_8(rd[0], rd[1])`
- `- RC_8_8(rd[2], rd[3])`

write to rd

- `- enabler * Memory(REG_AS, rd_idx, rd_prev_clk, rd_prev[0], rd_prev[1], rd_prev[2], rd_prev[3])`
- `+ enabler * Memory(REG_AS, rd_idx, clk, rd[0], rd[1], rd[2], rd[3])`
- `- RC_20(clk - rd_prev_clk)`

## 5. Less Than Reg (slt/sltu)

### 5.1 Columns

- pc
- clk

- rd_idx
- rd_prev_clk
- rd_prev[0..3]
- cmp_result

- rs1_idx
- rs1_prev_clk
- rs1[0..3]
- rs1_msl_felt

- rs2_idx
- rs2_prev_clk
- rs2[0..3]
- rs2_msl_felt

- opcode_slt_flag
- opcode_sltu_flag

- diff_marker[0..3]
- diff_val

### 5.2 Variables

- `enabler = opcode_slt_flag + opcode_sltu_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.

### 5.3 Constraints

`enabler`, `opcode_*_flags` and `cmp_result` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `diff_marker_i * (1 - diff_marker_i)`

read instruction from the Program segment (R-type)

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, rs2_idx)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_idx, rs1_prev_clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `+ enabler * Memory(REG_AS, rs1_idx, clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `- RC_20(clk - rs1_prev_clk)`

read from rs2

- `- enabler * Memory(REG_AS, rs2_idx, rs2_prev_clk, rs2[0], rs2[1], rs2[2], rs2[3])`
- `+ enabler * Memory(REG_AS, rs2_idx, clk, rs2[0], rs2[1], rs2[2], rs2[3])`
- `- RC_20(clk - rs2_prev_clk)`

`msl` are the most significant limbs as felts

- `(rs1[3] - rs1_msl_felt) * (2^N_BITS_PER_BYTE - (rs1[3] - rs1_msl_felt) )`
- `(rs2[3] - rs2_msl_felt) * (2^N_BITS_PER_BYTE - (rs2[3] - rs2_msl_felt) )`

comparison logic for each limb i (from 3 down to 0), `prefix_sum` is the running
sum of `diff_marker_i` and
`diff = (2 * cmp_result - 1) * ( if i == 3 {rs2_msl_felt - rs1_msl_felt} else {rs2[i] - rs1[i]} )`

- `(1 - prefix_sum) * diff`
- `diff_marker_i * (diff_val - diff)`

`prefix_sum` contains at most one activation (`prefix_sum = Σ diff_marker_i`)

- `prefix_sum * (1 - prefix_sum)`

if equal, result is 0

- `(1 - prefix_sum) * cmp_result`

range check msl felts with sign consideration (`opcode_slt_flag = 1`,
`cmp_result` = 0, `rs1[3] = 32`, `rs1_msl_felt = 32 - 256`, `rs2[3] = 64` and
`rs2_msl_felt = 64` would pass without this check)

- `- RC_8_8(rs1_msl_felt + opcode_slt_flag * 2^(N_BITS_PER_BYTE-1), rs2_msl_felt + opcode_slt_flag * 2^(N_BITS_PER_BYTE-1))`

diff_val is > 0

- `- prefix_sum * RC_8_8(diff_val - 1, 0)`

write to rd

- `- enabler * Memory(REG_AS, rd_idx, rd_prev_clk, rd_prev[0], rd_prev[1], rd_prev[2], rd_prev[3])`
- `+ enabler * Memory(REG_AS, rd_idx, clk, cmp_result, 0, 0, 0)`
- `- RC_20(clk - rd_prev_clk)`

## 6. Less Than Imm (slti/sltiu)

### 6.1 Columns

- pc
- clk

- rd_idx
- rd_prev_clk
- rd_prev[0..3]
- cmp_result

- rs1_idx
- rs1_prev_clk
- rs1[0..3]
- rs1_msl_felt

- imm_0 (imm[0:7])
- imm_1 (imm[8:10])
- imm_msb (imm[11])

- opcode_slti_flag
- opcode_sltiu_flag

- diff_marker[0..3]
- diff_val

### 6.2 Variables

- `enabler = opcode_slti_flag + opcode_sltiu_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `imm = imm_0 + imm_1 * 2^8 + imm_msb * 2^11`
- `sext_imm_0 = imm_0`
- `sext_imm_1 = imm_1 + 2^3 * (2^5 - 1) * imm_msb`
- `sext_imm_2 = (2^8 - 1) * imm_msb`
- `sext_imm_3 = (2^8 - 1) * imm_msb`
- `sext_imm_msl_felt = opcode_sltiu_flag * sext_imm_3 - opcode_slti_flag * imm_msb`

### 6.3 Constraints

`enabler` and `opcode_*_flags` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`

read instruction from the Program segment (I-type)

- `- enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, imm)`

range check imm and range check `rs1_msl_felt` with sign consideration

- `- RC_8_8_3(rs1_msl_felt + opcode_slti_flag * 2^(N_BITS_PER_BYTE-1), imm_0, imm_1)`
- `imm_msb * (1 - imm_msb)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_idx, rs1_prev_clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `+ enabler * Memory(REG_AS, rs1_idx, clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `- RC_20(clk - rs1_prev_clk)`

`msl` are the most significant limbs as felts

- `(rs1[3] - rs1_msl_felt) * (2^N_BITS_PER_BYTE - (rs1[3] - rs1_msl_felt) )`

diff markers are boolean and sum correctly

- `diff_marker_i * (1 - diff_marker_i)`
- `prefix_sum * (1 - prefix_sum)`

comparison logic for each limb i (from 3 down to 0), `prefix_sum` is the running
sum of `diff_marker_i` and
`diff = (2 * cmp_result - 1) * ( if i == 3 {sext_imm_msl_felt - rs1_msl_felt} else {sext_imm_i[i] - rs1[i]} )`

- `(1 - prefix_sum) * diff`
- `diff_marker_i * (diff_val - diff)`

`prefix_sum` contains at most one activation (`prefix_sum = Σ diff_marker_i`)

- `prefix_sum * (1 - prefix_sum)`

if equal, result is 0

- `(1 - prefix_sum) * cmp_result`

range check diff_val is non-zero when prefix_sum = 1

- `- prefix_sum * RC_8_8(diff_val - 1, 0)`

result is boolean

- `cmp_result * (1 - cmp_result)`

write to rd

- `- enabler * Memory(REG_AS, rd_idx, rd_prev_clk, rd_prev[0], rd_prev[1], rd_prev[2], rd_prev[3])`
- `+ enabler * Memory(REG_AS, rd_idx, clk, cmp_result, 0, 0, 0)`
- `- RC_20(clk - rd_prev_clk)`

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
- diff_inv_marker[0..3] - 0 everywhere but for i where `a[i] != b[i]` if such i
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

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(branch_target, clk + 1))`

read from rs1

- `- enabler * Memory(REG_AS, rs1_idx, rs1_prev_clk, a_0, a_1, a_2, a_3)`
- `+ enabler * Memory(REG_AS, rs1_idx, clk, a_0, a_1, a_2, a_3)`
- `- RC_20(clk - rs1_prev_clk - enabler)`

read from rs2

- `- enabler * Memory(REG_AS, rs2_idx, rs2_prev_clk, b_0, b_1, b_2, b_3)`
- `+ enabler * Memory(REG_AS, rs2_idx, clk, b_0, b_1, b_2, b_3)`
- `- (1 - in_place_flag) * RC_20(clk - rs2_prev_clk - enabler)`
- `in_place_flag * (clk - rs2_prev_clk)`

check `cmp_eq`

- for i in [0..3]: `cmp_eq * ( a[i] - b[i] )`
- `enabler * (1 - diff_inv_sum)`

## 8. Branch Less Than (blt/bltu/bge/bgeu)

### 8.1 Columns

- pc
- clk
- in_place_flag

- rs1_idx
- rs1_prev_clk
- a_0, a_1, a_2, a_3 — limbs of `rs1`.
- a_msb_f

- rs2_idx
- rs2_prev_clk
- b_0, b_1, b_2, b_3 — limbs of `rs2`
- b_msb_f

- imm - equals M31(imm) if imm>=0 and - M31(imm) if imm<0

- cmp_result - jump branch if cmp_result is 1
- cmp_lt - 1 if a < b, 0 otherwise
- diff_marker[0..3] - 0 everywhere but for i where `a[i] != b[i]` if such i
  exists, `diff_marker[i] = (a[i] - b[i])^-1`
- branch_target

- opcode_blt_flag
- opcode_bltu_flag
- opcode_bge_flag
- opcode_bgeu_flag

### 8.2 Variables

- `enabler = opcode_blt_flag + opcode_bltu_flag + opcode_bge_flag + opcode_bgeu_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `rs_idx_diff = rs1_idx - rs2_idx`
- `lt = opcode_blt_flag + opcode_bltu_flag`
- `ge = opcode_bge_flag + opcode_bgeu_flag`
- `signed = opcode_blt_flag + opcode_bge_flag`
- `a_diff = a_3 - a_msb_f`
- `b_diff = b_3 - b_msb_f`

### 8.3 Constraints

`enabler`, `opcode_*_flags`, `in_place_flag` and `cmp_result` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `in_place_flag * (1 - in_place_flag)`
- `cmp_result * (1 - cmp_result)`
- `diff_marker[i] * (1 - diff_marker[i])`

if in-place flag is 1 then rs1_idx == rs2_idx

- `in_place_flag * rs_idx_diff`

read instruction from the Program segment

- `- enabler * Program(pc, expected_opcode_id, rs1_idx, rs2_idx, imm)`

check branch target

- `branch_target - ( pc + imm * cmp_result + 4 * (1 - cmp_result) )`

registers update (conditional branch)

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(branch_target, clk + 1))`

read from rs1

- `- enabler * Memory(REG_AS, rs1_idx, rs1_prev_clk, a_0, a_1, a_2, a_3)`
- `+ enabler * Memory(REG_AS, rs1_idx, clk, a_0, a_1, a_2, a_3)`
- `- RC_20(clk - rs1_prev_clk - enabler)`

read from rs2

- `- enabler * Memory(REG_AS, rs2_idx, rs2_prev_clk, b_0, b_1, b_2, b_3)`
- `+ enabler * Memory(REG_AS, rs2_idx, clk, b_0, b_1, b_2, b_3)`
- `- (1 - in_place_flag) * RC_20(clk - rs2_prev_clk - enabler)`
- `in_place_flag * (clk - rs2_prev_clk)`

msb field elements must match actual msb bytes

- `a_diff * (2^N_BITS_PER_BYTE - a_diff)`
- `b_diff * (2^N_BITS_PER_BYTE - b_diff)`

comparison logic for each limb i (from 3 down to 0), `prefix_sum` is the running
sum of `diff_marker_i` and
`diff = (if i == 3 then b_msb_f - a_msb_f else b_i - a_i) * (2 * cmp_lt - 1)`

- `(1 - prefix_sum) * diff`
- `diff_marker_i * (diff_val - diff)`

`prefix_sum` contains at most one activation (`prefix_sum = Σ diff_marker_i`)

- `prefix_sum * (1 - prefix_sum)`

if equal, result is 0

- `(1 - prefix_sum) * cmp_lt`

range check msb field elements with sign consideration

- `- RC_8_8(a_msb_f + signed * 2^(N_BITS_PER_BYTE-1), b_msb_f + signed * 2^(N_BITS_PER_BYTE-1))`

range check diff_val is non-zero when prefix_sum = 1

- `- prefix_sum * RC_8_8(diff_val - 1, 0)`

check `cmp_lt`

- `cmp_lt - ( cmp_result * lt + (1 - cmp_result) * ge )`

## 9. LUI

### 9.1 Columns

- enabler
- pc
- clk

- rd_idx
- rd_prev_clk
- rd_prev[0..3]

- imm_0 (decoded_imm[0..3])
- imm_1 (decoded_imm[4:11])
- imm_2 (decoded_imm[12:19])

- branch_target

### 9.2 Variables

- `imm = imm_0 + imm_1 * 2^4 + imm_2 * 2^12`

### 9.3 Constraints

`enabler` is a boolean

- `enabler * (1 - enabler)`

read instruction from the Program segment (U-type)

- `- enabler * Program(pc, opcode_lui_id, rd_idx, imm, 0)`

registers update

- `- enabler * Registers(pc, clk)`
- `- enabler * Registers(pc + 4, clk + 1)`

range check rd:

- `- RC_4_8_8(imm_0, imm_1, imm_2)`

write to rd

- `- enabler * Memory(REG_AS, rd_idx, rd_prev[0], rd_prev[1], rd_prev[2], rd_prev[3])`
- `+ enabler * Memory(REG_AS, rd_idx, clk, 0, imm_0 * 2^4, imm_1, imm_2)`
- `- RC_20(clk - rd_prev_clk)`

## 10. AUIPC

### 10.1 Columns

- enabler
- pc
- clk

- rd_idx
- rd_prev_clk
- rd_prev[0..3]
- rd[0..3]

- imm_felt

### 10.2 Variables

- `rd_felt = rd[0] + rd[1] * 2^8 + rd[2] * 2^16 + rd[3] * 2^24`.

### 10.3 Constraints

`enabler` is a boolean

- `enabler * (1 - enabler)`

read instruction from the Program segment (U-type)

- `- enabler * Program(pc, opcode_auipc_id, rd_idx, imm_felt, 0)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

check that rd is pc + imm

- `rd_felt - (pc + imm_felt)`

range check rd

- `RC_8_8(rd[1], rd[2])`
- `RC_M31(rd[0], rd[3])`

write to rd

- `- enabler * Memory(REG_AS, rd_prev_clk, rd_prev[0], rd_prev[1], rd_prev[2], rd_prev[3])`
- `+ enabler * Memory(REG_AS, clk, rd[0], rd[1], rd[2], rd[3])`
- `- RC_20(clk - rd_prev_clk)`

## 11. JALR

### 11.1 Columns

- enabler
- pc
- to_pc_over_two
- to_pc_lsb
- clk

- rs1_prev_clk
- rs1_idx
- rs1[0..3]

- rd_prev_clk
- rd_idx
- rd_prev[0..3]
- rd[0..3]

- imm_felt

### 11.2 Variables

- `rs1_felt = rs1[0] + rs1[1] * 2^8 + rs1[2] * 2^16 + rs1[3] * 2^24`
- `rd_felt = rd[0] + rd[1] * 2^8 + rd[2] * 2^16 + rd[3] * 2^24`

### 11.3 Constraints

`enabler` and `to_pc_lsb` are boolean

- `enabler * (1 - enabler)`
- `to_pc_lsb * (1 - to_pc_lsb)`

read instruction from the Program segment (I-type)

- `- enabler * Program(pc, opcode_jalr_id, rd_idx, rs1_idx, imm_felt)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_idx, rs1_prev_clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `+ enabler * Memory(REG_AS, rs1_idx, clk, rs1[0], rs1[1], rs1[2], rs1[3])`
- `- RC_20(clk - rs1_prev_clk)`

check that rs1 is a M31

- `- RC_M31(rs1[0], rs1[3])`

check next pc

- `2 * to_pc_over_two + to_pc_lsb - (rs1_felt + imm_felt)`

update registers

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(2 * to_pc_over_two, clk + 1)`

check that rd is a M31

- `RC_8_8(rd[1], rd[2])`
- `RC_M31(rd[0], rd[3])`

rd is pc+4

- `rd_felt - (pc + 4)`

write to rd

- `- enabler * Memory(REG_AS, rd_idx, rd_prev_clk, rd_prev[0], rd_prev[1], rd_prev[2], rd_prev[3])`
- `+ enabler * Memory(REG_AS, rd_idx, clk, rd[0], rd[1], rd[2], rd[3])`
- `- RC_20(clk - rd_prev_clk)`

## 12. JAL

### 12.1 Columns

- enabler
- pc
- clk

- rd_prev_clk
- rd_idx
- rd_prev[0..3]
- rd[0..3]

- imm_felt

### 12.2 Variables

- `rd_felt = rd[0] + rd[1] * 2^8 + rd[2] * 2^16 + rd[3] * 2^24`

### 12.3 Constraints

`enabler` is a boolean

- `enabler * (1 - enabler)`

read instruction from the Program segment (U-type)

- `- enabler * Program(pc, opcode_jal_id, rd_idx, imm_felt, 0)`

update registers

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + imm, clk + 1)`

check that rd is a M31

- `RC_8_8(rd[1], rd[2])`
- `RC_M31(rd[0], rd[3])`

rd is pc+4

- `rd_felt - (pc + 4)`

write to rd

- `- enabler * Memory(REG_AS, rd_idx, rd_prev_clk, rd_prev[0], rd_prev[1], rd_prev[2], rd_prev[3])`
- `+ enabler * Memory(REG_AS, rd_idx, clk, rd[0], rd[1], rd[2], rd[3])`
- `- RC_20(clk - rd_prev_clk)`

## 13. Load/store (lb/lbu/lh/lhu/lw/sb/sh/sw)

### 13.0 Factorization cost

Extra cost compared to having 3 components: lbu/lhu/lw - lb/lh - sb/sh/sw

- for load-u : 5T (lb/lh/sb/sh/sw flags) + 2T (src/dst_addr) + 1T (src_msb)
- for load-s : 6T (lbu/lhu/lw/sb/sh/sw flags) + 2T (src/dst_addr)
- for store: 5T (lbu/lb/lhu/lh/lw) + 2T (src/dst_addr) + 1T (src_msb)

=> 8 unused cells per opcode

Extra cost compared to having 2 components: lbu/lhu/lw/lb/lh - sb/sh/sw

- for load : 3T (sb/sh/sw flags) + 2T (src/dst_addr)
- for store: 5T (lbu/lb/lhu/lh/lw) + 2T (src/dst_addr) + 1T (src_msb)

=> 8 unused cells per store and 5 per load

### 13.1 Columns

- pc
- clk

<!-- destination columns -->

- dst_addr
- dst_prev_clk (rd_prev_clk - mem_prev_clk)
- dst_prev_val[0..3] (rd_prev_val[0..3] - mem_prev_val[0..3])
- dst[0..3] (rd[0..3] - mem_val[0..3])

<!-- columns for byte/halfword/word address -->

- rs1_prev_clk
- rs1_idx
- base[0..3]
- imm_felt

<!-- second register index -->

- r2_idx (rd_idx - rs2_idx)

<!-- source columns (3rd byte of src val is src[3]+2^7*src_msb)-->

- src_addr
- src_prev_clk (mem_prev_clk - rs2_prev_clk)
- src[0..3] (mem_val[0..3] - rs2[0..3])
- src_msb

<!-- columns for address shifting -->

- shift_amount
- markers[0..3] - one-hot encoding of the loaded bytes position (LE)

<!-- flags -->

- opcode_lb_flag
- opcode_lh_flag
- opcode_lbu_flag
- opcode_lhu_flag
- opcode_lw_flag
- opcode_sb_flag
- opcode_sh_flag
- opcode_sw_flag

### 13.2 Variables

- `enabler = Σ opcode_i_flag`
- `expected_opcode_id =  Σ opcode_i_flag * opcode_id_i`
- `mem_addr = base[0] + base[1] * 2^8 + base[2] * 2^16 + base[3] * 2^24 + imm`
- `sum_marker = Σ marker[i]`
- `shift_id = Σ i * marker[i]`
- `opcode_b_flag = opcode_lbu_flag + opcode_lb_flag + opcode_sb_flag`
- `opcode_h_flag = opcode_lhu_flag + opcode_lh_flag + opcode_sh_flag`
- `opcode_w_flag = opcode_lwu_flag + opcode_sw_flag`
- `is_signed = opcode_lb_flag + opcode_lh_flag`
- `is_store = opcode_sb_flag + opcode_sh_flag + opcode_sw_flag`
- `is_load = 1 - is_store`
- `src_as = REG_AS * is_store + RW_AS * is_load`
- `dst_as = REG_AS * is_load + RW_AS * is_store`
- `src[3] = src[3] + src_msb * 2^7`

### 13.3 Constraints

`enabler`, `opcode_*_flags` and `marker[i]` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `marker[i] * (1 - marker[i])`

read instruction from the Program segment (I-type for loads and S-type for
stores)

- `- enabler * Program(pc, expected_opcode_id, rs1_idx, r2_idx, imm)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_idx, rs1_prev_clk, base[0], base[1], base[2], base[3])`
- `+ enabler * Memory(REG_AS, rs1_idx, clk, base[0], base[1], base[2], base[3])`
- `- RC_20(clk - rs1_prev_clk)`

check shift amount

- `shift_amount - ( opcode_b_flag * shift_id + opcode_h_flag * ( (shift_id - 1) / 2 ) + opcode_w_flag * 0 )`

check that `base[0] - shift_amount` is a multiple of 4

- `- RC_6((base[0] - shift_amount)/2^2)`

check that base is a M31:

- `- RC_M31(base[0], base[3])`

check src/dst addresses (load/store dependent)

- `src_addr - ( is_load * (mem_addr - shift_amount) + is_store * r2_idx )`
- `dst_addr - ( is_load * r2_idx + is_store * (mem_addr - shift_amount) )`

read src

- `- enabler * Memory(src_as, src_addr, src_prev_clk, src[0], src[1], src[2], src[3])`
- `+ enabler * Memory(src_as, src_addr, clk, src[0], src[1], src[2], src[3])`
- `- RC_20(clk - src_prev_clk)`

for lbu/sb `marker` contains a single one when row is enabled

- `opcode_b_flag * (1 - sum_marker)`

for lhu/sh `marker` is either `[1,1,0,0]` or `[0,0,1,1]`

- `opcode_h_flag * (2 - sum_marker)`
- `opcode_h_flag * (1 - shift_id) * (5 - shift_id)`

check that lbu/sb loads the correct byte

- `opcode_b_flag * (is_signed * src_msb * (2^8-1) - dst[1])`
- `opcode_b_flag * (is_signed * src_msb * (2^8-1) - dst[2])`
- `opcode_b_flag * (is_signed * src_msb * (2^8-1) - dst[3])`
- for i in [0..3] `opcode_b_flag * (dst[0] - src[i]) * marker[i]`

check that lhu/sh loads the correct half word

- `opcode_h_flag * (is_signed * src_msb * (2^8-1) - dst[2])`
- `opcode_h_flag * (is_signed * src_msb * (2^8-1) - dst[3])`
- `opcode_h_flag * ( (5 - shift_id) / 4 ) * (dst[0] - src[0])`
- `opcode_h_flag * ( (5 - shift_id) / 4 ) * (dst[1] - src[1])`
- `opcode_h_flag * ( (shift_id - 1) / 4 ) * (dst[0] - src[2])`
- `opcode_h_flag * ( (shift_id - 1) / 4 ) * (dst[1] - src[3])`

check that lw/sw loads all the bytes

- `opcode_w_flag * (dst[0] - src[0])`
- `opcode_w_flag * (dst[1] - src[1])`
- `opcode_w_flag * (dst[2] - src[2])`
- `opcode_w_flag * (dst[3] - src[3])`

write into dst

- `- enabler * Memory(dst_as, dst_addr, dst_prev_clk, dst_prev_val[0], dst_prev_val[1], dst_prev_val[2], dst_prev_val[3])`
- `+ enabler * Memory(dst_as, dst_addr, clk, dst[0], dst[1], dst[2], dst[3])`
- `- RC_20(clk - dst_prev_clk)`
