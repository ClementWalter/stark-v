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
- I-type: `(opcode_id, rd_idx, rs1_idx, imm_decoded)`
- I-type (shift): `(opcode_id, rd_idx, rs1_idx, imm_truncated)`
- I-type (load): `(opcode_id, rs1_idx, rd_idx, imm_felt)`
- U-type (lui): `(opcode_id, rd_idx, decoded_imm, 0)`
- U-type (auipc): `(opcode_id, rd_idx, imm_felt, 0)`
- J-type: `(opcode_id, rd_idx, imm_idx, 0)`
- B-type: `(opcode_id, rs1_idx, rs2_idx, imm)`

## 1. Base ALU Reg (add/sub/xor/or/and)

- `add`: `x[rd] = x[rs1] + x[rs2]`
- `sub`: `x[rd] = x[rs1] - x[rs2]`
- `xor`: `x[rd] = x[rs1] ^ x[rs2]`
- `or`: `x[rd] = x[rs1] | x[rs2]`
- `and`: `x[rd] = x[rs1] & x[rs2]`

### 1.0 Factorization cost

Extra cost compared to having 2 components: add/sub - xor/or/and

- for bitwise: 4T
- for add/sub: 4T + 2L (`max_log_size = 21`) or 4T + 4L (`max_log_size = 20`)

=> 4 unused cells per bitwise and 8 to 12 cells per add/sub

### 1.1 Columns

- pc
- clk

- rd_addr
- rd_prev_0..3
- rd_clk_prev
- rd_next_0..3

- rs1_addr
- rs1_prev_0..3
- rs1_clk_prev
- rs1_next_0..3

- rs2_addr
- rs2_prev_0..3
- rs2_clk_prev
- rs2_next_0..3

- opcode_add_flag
- opcode_sub_flag
- opcode_xor_flag
- opcode_or_flag
- opcode_and_flag

### 1.2 Variables

- `enabler = opcode_add_flag + opcode_sub_flag + opcode_xor_flag + opcode_or_flag + opcode_and_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `carry_add[0] = (rs1[0] + rs2[0] - rd[0]) / 2^8`.
- `carry_add[i] = (rs1[i] + rs2[i] + carry_add[i - 1] - rd[i]) / 2^8` for i in
  [1..3].
- `carry_sub[0] = (rd[0] + rs2[0] - rs1[0]) / 2^8`.
- `carry_sub[i] = (rd[i] + rs2[i] - rs1[i] + carry_sub[i - 1]) / 2^8` for i in
  [1..3].
- `is_bitwise = opcode_xor_flag + opcode_or_flag + opcode_and_flag`.
- `bitwise_id = opcode_or_flag + 2 * opcode_xor_flag`.

### 1.3 Constraints

`enabler` and `opcode_*_flags` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`

check carries

- `opcode_add_flag * carry_add[i] * (1 - carry_add[i])`
- `opcode_sub_flag * carry_sub[i] * (1 - carry_sub[i])`

read instruction from the Program segment (R-type)

- `- enabler * Program(pc, expected_opcode_id, rd_addr, rs1_addr, rs2_addr)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_addr, rs1_clk_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3)`
- `+ enabler * Memory(REG_AS, rs1_addr, clk, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3)`
- `- RC_20(clk - rs1_clk_prev)`

read from rs2

- `- enabler * Memory(REG_AS, rs2_addr, rs2_clk_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3)`
- `+ enabler * Memory(REG_AS, rs2_addr, clk, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3)`
- `- RC_20(clk - rs2_clk_prev)`

perform bitwise operation and RC rd (same tradeoff as 1.3)

- `- is_bitwise * Bitwise(rs1_next_0, rs2_next_0, rd_next_0, bitwise_id)`
- `- is_bitwise * Bitwise(rs1_next_1, rs2_next_1, rd_next_1, bitwise_id)`
- `- is_bitwise * Bitwise(rs1_next_2, rs2_next_2, rd_next_2, bitwise_id)`
- `- is_bitwise * Bitwise(rs1_next_3, rs2_next_3, rd_next_3, bitwise_id)`

range check rd

- `- RC_8_8(rd[0], rd[1])`
- `- RC_8_8(rd[2], rd[3])`

write to rd

- `- enabler * Memory(REG_AS, rd_addr, rd_clk_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3)`
- `+ enabler * Memory(REG_AS, rd_addr, clk, rd_next_0, rd_next_1, rd_next_2, rd_next_3)`
- `- RC_20(clk - rd_clk_prev)`

## 2. Base ALU Imm (addi/xori/ori/andi)

- `addi`: `x[rd] = x[rs1] + sext(immediate)`
- `xori`: `x[rd] = x[rs1] ^ sext(immediate)`
- `ori`: `x[rd] = x[rs1] | sext(immediate)`
- `andi`: `x[rd] = x[rs1] & sext(immediate)`

### 2.0 Factorization cost

Same as 1.0

### 2.1 Columns

- pc
- clk

- rd_addr
- rd_prev_0..3
- rd_clk_prev
- rd_next_0..3

- rs1_addr
- rs1_prev_0..3
- rs1_clk_prev
- rs1_next_0..3

- imm_0 (imm[0:7])
- imm_1 (imm[8:10])
- imm_msb (imm[11])

- opcode_add_flag
- opcode_xor_flag
- opcode_or_flag
- opcode_and_flag

### 2.2 Variables

- `enabler = opcode_add_flag + opcode_xor_flag + opcode_or_flag + opcode_and_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `imm = imm_0 + imm_1 * 2^8 + imm_msb * 2^11`
- `sext_imm_0 = imm_0`
- `sext_imm_1 = imm_1 + 2^3 * (2^5 - 1) * imm_msb`
- `sext_imm_2 = (2^8 - 1) * imm_msb`
- `sext_imm_3 = (2^8 - 1) * imm_msb`
- `carry_add[0] = (rs1[0] + sext_imm_0 - rd[0]) / 2^8`
- `carry_add[i] = (rs1[i] + sext_imm_i + carry_add[i - 1] - rd[i]) / 2^8` for i
  in [1..3]
- `is_bitwise = opcode_xor_flag + opcode_or_flag + opcode_and_flag`
- `bitwise_id =  opcode_or_flag + 2 * opcode_xor_flag`.

### 2.3 Constraints

`enabler` and `opcode_*_flags` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`

`imm_msb` is boolean

- `imm_msb * (1 - imm_msb)`

check carries

- `opcode_add_flag * carry_add[i] * (1 - carry_add[i])`

read instruction from the Program segment (I-type)

- `- enabler * Program(pc, expected_opcode_id, rd_addr, rs1_addr, imm)`

range check imm (range checks sext_imm too)

- `- RC_8_3(imm_0, imm_1)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_addr, rs1_clk_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3)`
- `+ enabler * Memory(REG_AS, rs1_addr, clk, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3)`
- `- RC_20(clk - rs1_clk_prev)`

perform bitwise operation

- `- is_bitwise * Bitwise(rs1_next_0, sext_imm_0, rd_next_0, bitwise_id)`
- `- is_bitwise * Bitwise(rs1_next_1, sext_imm_1, rd_next_1, bitwise_id)`
- `- is_bitwise * Bitwise(rs1_next_2, sext_imm_2, rd_next_2, bitwise_id)`
- `- is_bitwise * Bitwise(rs1_next_3, sext_imm_3, rd_next_3, bitwise_id)`

range check rd (redundant for bitwise)

- `- RC_8_8(rd_next_0, rd_next_1)`
- `- RC_8_8(rd_next_2, rd_next_3)`

write to rd

- `- enabler * Memory(REG_AS, rd_addr, rd_clk_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3)`
- `+ enabler * Memory(REG_AS, rd_addr, clk, rd_next_0, rd_next_1, rd_next_2, rd_next_3)`
- `- RC_20(clk - rd_clk_prev)`

## 3. Shifts Reg (sll/srl/sra)

- `sll`: `x[rd] = x[rs1] << (x[rs2] & 0x1f)`
- `srl`: `x[rd] = x[rs1] >>u (x[rs2] & 0x1f)`
- `sra`: `x[rd] = x[rs1] >>s (x[rs2] & 0x1f)`

### 3.1 Columns

- pc
- clk

- rd_addr
- rd_prev_0..3
- rd_clk_prev
- rd_next_0..3

- rs1_addr
- rs1_prev_0..3
- rs1_clk_prev
- rs1_next_0..3
- rs1_sign

- rs2_addr
- rs2_prev_0..3
- rs2_clk_prev
- rs2_next_0..3

- opcode_sll_flag
- opcode_srl_flag
- opcode_sra_flag

- bit_multiplier_left
- bit_multiplier_right
- bit_shift_marker_0..7
- limb_shift_marker_0..3
- bit_shift_carry_0..3

### 3.2 Variables

- `enabler = opcode_sll_flag + opcode_srl_flag + opcode_sra_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`
- `left_shift = opcode_sll_flag`
- `right_shift = opcode_srl_flag + opcode_sra_flag`.
- `bit_multiplier = Σ 2^i * bit_shift_marker[i]`
- `bit_shift = Σ i * bit_shift_marker[i]`.
- `limb_shift = Σ i * limb_shift_marker[i]`.
- `shift_amount = limb_shift * 2^3 + bit_shift`.
- `rs1_msl = rs1[3] + 2^7 * rs1_sign`.

### 3.3 Constraints

`enabler`, `opcode_*_flags` and `rs1_sign` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `rs1_sign * (1 - rs1_sign)`

hot-one encodings must contain at most one activation

- `bit_shift_marker[i] * (1 - bit_shift_marker[i])`
- `limb_shift_marker[i] * (1 - limb_shift_marker[i])`
- `Σ bit_shift_marker[i] = enabler`
- `Σ limb_shift_marker[i] = enabler`

bit_multipliers are correctly formed

- `bit_multiplier_left - left_shift * bit_multiplier`
- `bit_multiplier_right - right_shift * bit_multiplier`

left shift constraints, for i in [0..3] and for j in [0..3]:

- `left_shift * limb_shift_marker[i] * rd[j]` for `j < i`.
- `left_shift * limb_shift_marker[i] * (rd[j] + 2^8 * bit_shift_carry[j - i]) - limb_shift_marker[i] * rs1[j - i] * bit_multiplier_left`
  for `j == i`
- `left_shift * limb_shift_marker[i] * (rd[j] - (bit_shift_carry[j - i - 1] - 2^8 * bit_shift_carry[j - i])) - limb_shift_marker[i] * rs1[j - i] * bit_multiplier_left`
  for `j > i`.

right shift constraints, for i in [0..3] and for j in [0..3]:

- `right_shift * limb_shift_marker[i] * (rd[j] - rs1_sign * (2^8 - 1))` for
  `j > 3 - i`.
- `limb_shift_marker[i] * (rs1_sign * (bit_multiplier_right - 1) * 2^8 + right_shift * (rs1[j + i] - bit_shift_carry[j + i]) - rd[j] * bit_multiplier_right)`
  for `j == 3 - i`
- `limb_shift_marker[i] * (bit_shift_carry[j + i + 1] * right_shift * 2^8 + right_shift * (rs1[j + i] - bit_shift_carry[j + i]) - rd[j] * bit_multiplier_right)`
  for `j < 3 - i`

read instruction from the Program segment (R-type)

- `- enabler * Program(pc, expected_opcode_id, rd_addr, rs1_addr, rs2_addr)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_addr, rs1_clk_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3)`
- `+ enabler * Memory(REG_AS, rs1_addr, clk, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3)`
- `- RC_20(clk - rs1_clk_prev)`

read from rs2

- `- enabler * Memory(REG_AS, rs2_addr, rs2_clk_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3)`
- `+ enabler * Memory(REG_AS, rs2_addr, clk, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3)`
- `- RC_20(clk - rs2_clk_prev)`

the 5 first bits of `rs2_next_0` shift `limb_shift` full limbs and `bit_shift`
bits

- `- RC_20(2^3 - 1 - (rs2_next_0 - shift_amount) / 2^5)`

shift carries should not exceed `2^bit_shift`

- `- RC_8_8(bit_multiplier - enabler - bit_shift_carry_0, bit_multiplier - enabler - bit_shift_carry_1)`
- `- RC_8_8(bit_multiplier - enabler - bit_shift_carry_2, bit_multiplier - enabler - bit_shift_carry_3)`

range check rd

- `- RC_8_8(rd_next_0, rd_next_1)`
- `- RC_8_8(rd_next_2, rd_next_3)`

write to rd

- `- enabler * Memory(REG_AS, rd_addr, rd_clk_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3)`
- `+ enabler * Memory(REG_AS, rd_addr, clk, rd_next_0, rd_next_1, rd_next_2, rd_next_3)`
- `- RC_20(clk - rd_clk_prev)`

## 4. Shifts Imm (slli/srli/srai)

- `slli`: `x[rd] = x[rs1] << immediate[4:0]`
- `srli`: `x[rd] = x[rs1] >>u immediate[4:0]`
- `srai`: `x[rd] = x[rs1] >>s immediate[4:0]`

### 4.1 Columns

- pc
- clk

- rd_addr
- rd_prev_0..3
- rd_clk_prev
- rd_next_0..3

- rs1_addr
- rs1_prev_0..3
- rs1_clk_prev
- rs1_next_0..3
- rs1_sign

- imm_truncated (imm[0:4])

- opcode_sll_flag
- opcode_srl_flag
- opcode_sra_flag

- bit_multiplier_left
- bit_multiplier_right
- bit_shift_marker_0..7
- limb_shift_marker_0..3
- bit_shift_carry_0..3

### 4.2 Variables

- `enabler = opcode_sll_flag + opcode_srl_flag + opcode_sra_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`
- `left_shift = opcode_sll_flag`
- `right_shift = opcode_srl_flag + opcode_sra_flag`.
- `bit_multiplier = Σ 2^i * bit_shift_marker[i]`
- `bit_shift = Σ i * bit_shift_marker[i]`.
- `limb_shift = Σ i * limb_shift_marker[i]`.
- `shift_amount = limb_shift * 2^3 + bit_shift`.
- `rs1_msl = rs1[3] + 2^7 * rs1_sign`.

### 4.3 Constraints

`enabler`, `opcode_*_flags` and `rs1_sign` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `rs1_sign * (1 - rs1_sign)`

hot-one encodings must contain at most one activation

- `bit_shift_marker[i] * (1 - bit_shift_marker[i])`
- `limb_shift_marker[i] * (1 - limb_shift_marker[i])`
- `Σ bit_shift_marker[i] = enabler`
- `Σ limb_shift_marker[i] = enabler`

bit_multipliers are correctly formed

- `bit_multiplier_left - left_shift * bit_multiplier`
- `bit_multiplier_right - right_shift * bit_multiplier`

`imm_truncated` shifts `limb_shift` full limbs and `bit_shift` bits

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
- `limb_shift_marker[i] * (rs1_sign * (bit_multiplier_right - 1) * 2^8 + right_shift * (rs1[j + i] - bit_shift_carry[j + i]) - rd[j] * bit_multiplier_right)`
  for `j == 3 - i`
- `limb_shift_marker[i] * (bit_shift_carry[j + i + 1] * right_shift * 2^8 + right_shift * (rs1[j + i] - bit_shift_carry[j + i]) - rd[j] * bit_multiplier_right)`
  for `j < 3 - i`

read instruction from the Program segment (I-type)

- `- enabler * Program(pc, expected_opcode_id, rd_addr, rs1_addr, imm_truncated)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_addr, rs1_clk_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3)`
- `+ enabler * Memory(REG_AS, rs1_addr, clk, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3)`
- `- RC_20(clk - rs1_clk_prev)`

shift carries should not exceed `2^bit_shift`

- `- RC_8_8(bit_multiplier - enabler - bit_shift_carry_0, bit_multiplier - enabler - bit_shift_carry_1)`
- `- RC_8_8(bit_multiplier - enabler - bit_shift_carry_2, bit_multiplier - enabler - bit_shift_carry_3)`

range check rd

- `- RC_8_8(rd_next_0, rd_next_1)`
- `- RC_8_8(rd_next_2, rd_next_3)`

write to rd

- `- enabler * Memory(REG_AS, rd_addr, rd_clk_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3)`
- `+ enabler * Memory(REG_AS, rd_addr, clk, rd_next_0, rd_next_1, rd_next_2, rd_next_3)`
- `- RC_20(clk - rd_clk_prev)`

## 5. Less Than Reg (slt/sltu)

- `slt`: `x[rd] = 1 if x[rs1] <s x[rs2], 0 otherwise`
- `sltu`: `x[rd] = 1 if x[rs1] <u x[rs2], 0 otherwise`

### 5.1 Columns

- pc
- clk

- rd_addr
- rd_prev_0..3
- rd_clk_prev
- rd_next_0..3

- rs1_addr
- rs1_prev_0..3
- rs1_clk_prev
- rs1_next_0..3
- rs1_msl_felt

- rs2_addr
- rs2_prev_0..3
- rs2_clk_prev
- rs2_next_0..3
- rs2_msl_felt

- cmp_result

- opcode_slt_flag
- opcode_sltu_flag

- diff_marker_0..3
- diff_val

### 5.2 Variables

- `enabler = opcode_slt_flag + opcode_sltu_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `diff_markers = [diff_marker_0..3]`.

### 5.3 Constraints

`enabler`, `opcode_*_flags`, `cmp_result`, and `diff_marker[i]` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `cmp_result * (1 - cmp_result)`
- `diff_marker[i] * (1 - diff_marker[i])`

msl felts match the most-significant limb

- `rs1_msl_gap = rs1[3] - rs1_msl_felt`
- `rs1_msl_gap * (2^8 - rs1_msl_gap)`
- `rs2_msl_gap = rs2[3] - rs2_msl_felt`
- `rs2_msl_gap * (2^8 - rs2_msl_gap)`

comparison logic for each limb i (from 3 down to 0), `prefix_sum` is the running
sum of `diff_marker_i` and
`diff = (2 * cmp_result - 1) * ( if i == 3 {rs2_msl_felt - rs1_msl_felt} else {rs2[i] - rs1[i]} )`

- `(1 - prefix_sum) * diff`
- `diff_marker_i * (diff_val - diff)`

`prefix_sum` contains at most one activation (`prefix_sum = Σ diff_marker_i`)

- `prefix_sum * (1 - prefix_sum)`

if equal, result is 0

- `(1 - prefix_sum) * cmp_result`

read instruction from the Program segment (R-type)

- `- enabler * Program(pc, expected_opcode_id, rd_addr, rs1_addr, rs2_addr)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_addr, rs1_clk_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3)`
- `+ enabler * Memory(REG_AS, rs1_addr, clk, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3)`
- `- RC_20(clk - rs1_clk_prev)`

read from rs2

- `- enabler * Memory(REG_AS, rs2_addr, rs2_clk_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3)`
- `+ enabler * Memory(REG_AS, rs2_addr, clk, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3)`
- `- RC_20(clk - rs2_clk_prev)`

range check msl felts with sign consideration

- `- RC_8_8(rs1_msl_felt + opcode_slt_flag * 2^(8-1), rs2_msl_felt + opcode_slt_flag * 2^(8-1))`

diff_val is > 0

- `- prefix_sum * RC_20(diff_val - 1)`

write to rd

- `- enabler * Memory(REG_AS, rd_addr, rd_clk_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3)`
- `+ enabler * Memory(REG_AS, rd_addr, clk, rd_next_0, rd_next_1, rd_next_2, rd_next_3)`
- `- RC_20(clk - rd_clk_prev)`

## 6. Less Than Imm (slti/sltiu)

- `slti`: `x[rd] = 1 if x[rs1] <s sext(imm), 0 otherwise`
- `sltiu`: `x[rd] = 1 if x[rs1] <u sext(imm), 0 otherwise`

### 6.1 Columns

- pc
- clk

- rd_addr
- rd_prev_0..3
- rd_clk_prev
- rd_next_0..3

- rs1_addr
- rs1_prev_0..3
- rs1_clk_prev
- rs1_next_0..3
- rs1_msl_felt

- cmp_result

- imm_0 (imm[0:7])
- imm_1 (imm[8:10])
- imm_msb (imm[11])

- opcode_slti_flag
- opcode_sltiu_flag

- diff_marker_0..3
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
- `diff_markers = [diff_marker_0..3]`.

### 6.3 Constraints

`enabler` and `opcode_*_flags` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`

`imm_msb` is boolean

- `imm_msb * (1 - imm_msb)`

msl felts match the most-significant limb

- `rs1_msl_gap = rs1[3] - rs1_msl_felt`
- `rs1_msl_gap * (2^8 - rs1_msl_gap)`

`diff_marker[i]` are booleans

- `diff_marker[i] * (1 - diff_marker[i])`

comparison logic for each limb i (from 3 down to 0), `prefix_sum` is the running
sum of `diff_marker_i` and
`diff = (2 * cmp_result - 1) * ( if i == 3 {sext_imm_msl_felt - rs1_msl_felt} else {sext_imm[i] - rs1[i]} )`

- `(1 - prefix_sum) * diff`
- `diff_marker_i * (diff_val - diff)`

`prefix_sum` contains at most one activation (`prefix_sum = Σ diff_marker_i`)

- `prefix_sum * (1 - prefix_sum)`

if equal, result is 0

- `(1 - prefix_sum) * cmp_result`

`cmp_result` is boolean

- `cmp_result * (1 - cmp_result)`

read instruction from the Program segment (I-type)

- `- enabler * Program(pc, expected_opcode_id, rd_addr, rs1_addr, imm)`

range check imm and range check `rs1_msl_felt` with sign consideration

- `- RC_8_8_3(rs1_msl_felt + opcode_slti_flag * 2^(8-1), imm_0, imm_1)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_addr, rs1_clk_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3)`
- `+ enabler * Memory(REG_AS, rs1_addr, clk, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3)`
- `- RC_20(clk - rs1_clk_prev)`

range check diff_val is non-zero when prefix_sum = 1

- `- prefix_sum * RC_20(diff_val - 1)`

write to rd

- `- enabler * Memory(REG_AS, rd_addr, rd_clk_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3)`
- `+ enabler * Memory(REG_AS, rd_addr, clk, rd_next_0, rd_next_1, rd_next_2, rd_next_3)`
- `- RC_20(clk - rd_clk_prev)`

## 7. Branch Equal (beq/bne)

- `beq`: `if (x[rs1] == x[rs2]) pc += sext(offset)`
- `bne`: `if (x[rs1] != x[rs2]) pc += sext(offset)`

### 7.1 Columns

- pc
- clk

- rs1_addr
- rs1_prev_0..3
- rs1_clk_prev
- rs1_next_0..3

- rs2_addr
- rs2_prev_0..3
- rs2_clk_prev
- rs2_next_0..3

- imm_felt

- cmp_result
- diff_inv_marker_0..3

- opcode_beq_flag
- opcode_bne_flag

### 7.2 Variables

- `enabler = opcode_beq_flag + opcode_bne_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `cmp_eq = cmp_result * opcode_beq_flag + (1 - cmp_result) * opcode_bne_flag`
- `diff_inv_sum = cmp_eq + Σ (rs1[i] - rs2[i]) * diff_inv_marker[i]`

### 7.3 Constraints

`enabler`, `opcode_*_flags` and `cmp_result` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `cmp_result * (1 - cmp_result)`

check `cmp_eq`

- for i in [0..3]: `cmp_eq * (rs1[i] - rs2[i])`
- `enabler * (1 - diff_inv_sum)`

read instruction from the Program segment (B-type)

- `- enabler * Program(pc, expected_opcode_id, rs1_addr, rs2_addr, imm_felt)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_addr, rs1_clk_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3)`
- `+ enabler * Memory(REG_AS, rs1_addr, clk, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3)`
- `- RC_20(clk - rs1_clk_prev)`

read from rs2

- `- enabler * Memory(REG_AS, rs2_addr, rs2_clk_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3)`
- `+ enabler * Memory(REG_AS, rs2_addr, clk, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3)`
- `- RC_20(clk - rs2_clk_prev)`

registers update (conditional branch), since there's an odd number of lookups,
we can `to_pc` with degree 2 by putting it at the end

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + imm_felt * cmp_result + 4 * (1 - cmp_result), clk + 1)`

## 8. Branch Less Than (blt/bltu/bge/bgeu)

- `blt`: `if (x[rs1] <s x[rs2]) pc += sext(offset)`
- `bltu`: `if (x[rs1] <u x[rs2]) pc += sext(offset)`
- `bge`: `if (x[rs1] >=s x[rs2]) pc += sext(offset)`
- `bgeu`: `if (x[rs1] >=u x[rs2]) pc += sext(offset)`

### 8.1 Columns

- pc
- clk

- rs1_addr
- rs1_prev_0..3
- rs1_clk_prev
- rs1_next_0..3
- rs1_msl_felt

- rs2_addr
- rs2_prev_0..3
- rs2_clk_prev
- rs2_next_0..3
- rs2_msl_felt

- imm_felt

- cmp_result
- cmp_lt

- diff_marker_0..3
- diff_val
- branch_target

- opcode_blt_flag
- opcode_bltu_flag
- opcode_bge_flag
- opcode_bgeu_flag

### 8.2 Variables

- `enabler = opcode_blt_flag + opcode_bltu_flag + opcode_bge_flag + opcode_bgeu_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `lt = opcode_blt_flag + opcode_bltu_flag`
- `ge = opcode_bge_flag + opcode_bgeu_flag`
- `signed = opcode_blt_flag + opcode_bge_flag`

### 8.3 Constraints

`enabler`, `opcode_*_flags`, `cmp_result`, and `diff_marker[i]` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `cmp_result * (1 - cmp_result)`
- `diff_marker[i] * (1 - diff_marker[i])`

check branch target

- `enabler * (branch_target - (pc + imm_felt * cmp_result + 4 * (1 - cmp_result)))`

msl felts match the most-significant limb

- `rs1_msl_gap = rs1[3] - rs1_msl_felt`
- `rs1_msl_gap * (2^8 - rs1_msl_gap)`
- `rs2_msl_gap = rs2[3] - rs2_msl_felt`
- `rs2_msl_gap * (2^8 - rs2_msl_gap)`

comparison logic for each limb i (from 3 down to 0), `prefix_sum` is the running
sum of `diff_marker_i` and
`diff = (2 * cmp_lt - 1) * ( if i == 3 {rs2_msl_felt - rs1_msl_felt} else {rs2[i] - rs1[i]} )`

- `(1 - prefix_sum) * diff`
- `diff_marker_i * (diff_val - diff)`

`prefix_sum` contains at most one activation (`prefix_sum = Σ diff_marker_i`)

- `prefix_sum * (1 - prefix_sum)`

if equal, result is 0

- `(1 - prefix_sum) * cmp_lt`

check `cmp_lt`

- `cmp_lt - (cmp_result * lt + (1 - cmp_result) * ge)`

read instruction from the Program segment (B-type)

- `- enabler * Program(pc, expected_opcode_id, rs1_addr, rs2_addr, imm_felt)`

registers update (conditional branch)

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(branch_target, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_addr, rs1_clk_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3)`
- `+ enabler * Memory(REG_AS, rs1_addr, clk, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3)`
- `- RC_20(clk - rs1_clk_prev)`

read from rs2

- `- enabler * Memory(REG_AS, rs2_addr, rs2_clk_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3)`
- `+ enabler * Memory(REG_AS, rs2_addr, clk, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3)`
- `- RC_20(clk - rs2_clk_prev)`

range check msl felts with sign consideration

- `- RC_8_8(rs1_msl_felt + signed * 2^(8-1), rs2_msl_felt + signed * 2^(8-1))`

diff_val is > 0

- `- prefix_sum * RC_20(diff_val - 1)`

## 9. LUI

- `lui`: `x[rd] = sext(immediate[31:12] << 12)`

### 9.1 Columns

- enabler
- pc
- clk

- rd_addr
- rd_prev_0..3
- rd_clk_prev
- rd_next_0..3

- imm_0 (decoded_imm[0..3])
- imm_1 (decoded_imm[4:11])
- imm_2 (decoded_imm[12:19])

### 9.2 Variables

- `imm = imm_0 + imm_1 * 2^4 + imm_2 * 2^12`

### 9.3 Constraints

`enabler` is a boolean

- `enabler * (1 - enabler)`

read instruction from the Program segment (U-type)

- `- enabler * Program(pc, opcode_lui_id, rd_addr, imm, 0)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

range check imm limbs

- `- RC_8_8_4(imm_1, imm_2, imm_0)`

write to rd

- `- enabler * Memory(REG_AS, rd_addr, rd_clk_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3)`
- `+ enabler * Memory(REG_AS, rd_addr, clk, rd_next_0, rd_next_1, rd_next_2, rd_next_3)`
- `- RC_20(clk - rd_clk_prev)`

## 10. AUIPC

- `auipc`: `x[rd] = pc + sext(immediate[31:12] << 12)`

### 10.1 Columns

- enabler
- pc
- clk

- rd_addr
- rd_prev_0..3
- rd_clk_prev
- rd_next_0..3

- imm_felt

### 10.2 Variables

- `rd_felt = rd[0] + rd[1] * 2^8 + rd[2] * 2^16 + rd[3] * 2^24`.

### 10.3 Constraints

`enabler` is a boolean

- `enabler * (1 - enabler)`

check that rd is pc + imm

- `rd_felt - (pc + imm_felt)`

read instruction from the Program segment (U-type)

- `- enabler * Program(pc, opcode_auipc_id, rd_addr, imm_felt, 0)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

range check rd

- `- RC_8_8(rd_next_1, rd_next_2)`
- `- RC_M31(rd_next_0, rd_next_3)`

write to rd

- `- enabler * Memory(REG_AS, rd_addr, rd_clk_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3)`
- `+ enabler * Memory(REG_AS, rd_addr, clk, rd_next_0, rd_next_1, rd_next_2, rd_next_3)`
- `- RC_20(clk - rd_clk_prev)`

## 11. JALR

- `jalr`: `x[rd] = pc + 4; pc = (x[rs1] + sext(offset)) & ~1`

### 11.1 Columns

- enabler
- pc
- clk

- rd_addr
- rd_prev_0..3
- rd_clk_prev
- rd_next_0..3

- rs1_addr
- rs1_prev_0..3
- rs1_clk_prev
- rs1_next_0..3

- to_pc_over_two
- to_pc_lsb
- imm_felt

### 11.2 Variables

- `rs1_felt = rs1[0] + rs1[1] * 2^8 + rs1[2] * 2^16 + rs1[3] * 2^24`
- `rd_felt = rd[0] + rd[1] * 2^8 + rd[2] * 2^16 + rd[3] * 2^24`

### 11.3 Constraints

`enabler` and `to_pc_lsb` are boolean

- `enabler * (1 - enabler)`
- `to_pc_lsb * (1 - to_pc_lsb)`

check next pc

- `2 * to_pc_over_two + to_pc_lsb - (rs1_felt + imm_felt)`

rd is pc + 4

- `enabler * (rd_felt - (pc + 4))`

read instruction from the Program segment (I-type)

- `- enabler * Program(pc, opcode_jalr_id, rd_addr, rs1_addr, imm_felt)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_addr, rs1_clk_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3)`
- `+ enabler * Memory(REG_AS, rs1_addr, clk, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3)`
- `- RC_20(clk - rs1_clk_prev)`

check that rs1 is a M31

- `- RC_M31(rs1_next_0, rs1_next_3)`

update registers

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(2 * to_pc_over_two, clk + 1)`

check that rd is a M31

- `- RC_8_8(rd_next_1, rd_next_2)`
- `- RC_M31(rd_next_0, rd_next_3)`

write to rd

- `- enabler * Memory(REG_AS, rd_addr, rd_clk_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3)`
- `+ enabler * Memory(REG_AS, rd_addr, clk, rd_next_0, rd_next_1, rd_next_2, rd_next_3)`
- `- RC_20(clk - rd_clk_prev)`

## 12. JAL

- `jal`: `x[rd] = pc + 4; pc = pc + sext(offset)`

### 12.1 Columns

- enabler
- pc
- clk

- rd_addr
- rd_prev_0..3
- rd_clk_prev
- rd_next_0..3

- imm_felt

### 12.2 Variables

- `rd_felt = rd[0] + rd[1] * 2^8 + rd[2] * 2^16 + rd[3] * 2^24`

### 12.3 Constraints

`enabler` is a boolean

- `enabler * (1 - enabler)`

rd is pc + 4 (skip when rd_addr = 0)

- `enabler * rd_addr * (rd_felt - (pc + 4))`

read instruction from the Program segment (U-type)

- `- enabler * Program(pc, opcode_jal_id, rd_addr, imm_felt, 0)`

update registers

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + imm_felt, clk + 1)`

check that rd is a M31

- `- RC_8_8(rd_next_1, rd_next_2)`
- `- RC_M31(rd_next_0, rd_next_3)`

write to rd

- `- enabler * Memory(REG_AS, rd_addr, rd_clk_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3)`
- `+ enabler * Memory(REG_AS, rd_addr, clk, rd_next_0, rd_next_1, rd_next_2, rd_next_3)`
- `- RC_20(clk - rd_clk_prev)`

## 13. Load/store (lb/lbu/lh/lhu/lw/sb/sh/sw)

- `lb`: `x[rd] = sext(Mem[x[rs1] + offset][7:0])`
- `lbu`: `x[rd] = Mem[x[rs1] + offset][7:0]`
- `lh`: `x[rd] = sext(Mem[x[rs1] + offset][15:0])`
- `lhu`: `x[rd] = Mem[x[rs1] + offset][15:0]`
- `lw`: `x[rd] = Mem[x[rs1] + offset][31:0]`
- `sb`: `Mem[x[rs1] + offset][7:0] = x[rs2][7:0]`
- `sh`: `Mem[x[rs1] + offset][15:0] = x[rs2][15:0]`
- `sw`: `Mem[x[rs1] + offset][31:0] = x[rs2][31:0]`

### 13.1 Columns

- pc
- clk

- dst_addr
- dst_prev_0..3
- dst_clk_prev
- dst_next_0..3

- rs1_addr
- rs1_prev_0..3
- rs1_clk_prev
- rs1_next_0..3

- src_addr
- src_prev_0..3
- src_clk_prev
- src_next_0..3

- r2_idx
- imm_felt
- src_msb
- shift_amount
- src_addr_selector
- dst_addr_selector
- marker_0
- marker_1
- marker_2
- marker_3

- opcode_lb_flag
- opcode_lh_flag
- opcode_lbu_flag
- opcode_lhu_flag
- opcode_lw_flag
- opcode_sb_flag
- opcode_sh_flag
- opcode_sw_flag

### 13.2 Variables

- `enabler = opcode_lb_flag + opcode_lh_flag + opcode_lbu_flag + opcode_lhu_flag + opcode_lw_flag + opcode_sb_flag + opcode_sh_flag + opcode_sw_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `mem_addr = rs1 + imm_felt` (rs1 as 4 limbs, little-endian).
- `sum_markers = Σ marker_i`.
- `shift_id = Σ i * marker_i`.
- `opcode_b_flag = opcode_lbu_flag + opcode_lb_flag + opcode_sb_flag`.
- `opcode_h_flag = opcode_lhu_flag + opcode_lh_flag + opcode_sh_flag`.
- `opcode_w_flag = opcode_lw_flag + opcode_sw_flag`.
- `is_signed = opcode_lb_flag + opcode_lh_flag`.
- `load_b_flag = opcode_lb_flag + opcode_lbu_flag`.
- `load_h_flag = opcode_lh_flag + opcode_lhu_flag`.
- `is_store = opcode_sb_flag + opcode_sh_flag + opcode_sw_flag`.
- `is_load = enabler - is_store`.
- `src_as = REG_AS * is_store + RW_AS * is_load`.
- `dst_as = REG_AS * is_load + RW_AS * is_store`.

### 13.3 Constraints

`enabler`, `opcode_*_flags` and `marker_i` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `marker_i * (1 - marker_i)`

check shift amount

- `shift_amount - (opcode_b_flag * shift_id + opcode_h_flag * (shift_id - 1) / 2)`

check src/dst address selectors

- `src_addr_selector - (is_load * (mem_addr - shift_amount) + is_store * r2_idx)`
- `dst_addr_selector - (is_load * r2_idx + is_store * (mem_addr - shift_amount))`

marker structure

- `opcode_b_flag * (1 - sum_markers)`
- `opcode_h_flag * (2 - sum_markers)`
- `opcode_h_flag * (1 - shift_id) * (5 - shift_id)`

byte loads/stores (sign extension for loads only)

- `load_b_flag * (signed_mask - dst[1])`
- `load_b_flag * (signed_mask - dst[2])`
- `load_b_flag * (signed_mask - dst[3])`
- `load_b_flag * (dst[0] - src[i]) * marker_i`
- `opcode_sb_flag * (dst[i] - src[0]) * marker_i`

half-word loads/stores (sign extension for loads only)

- `load_h_flag * (signed_mask - dst[2])`
- `load_h_flag * (signed_mask - dst[3])`
- `load_h_flag * (5 - shift_id) / 4 * (dst[0] - src[0])`
- `load_h_flag * (5 - shift_id) / 4 * (dst[1] - src[1])`
- `load_h_flag * (shift_id - 1) / 4 * (dst[0] - src[2])`
- `load_h_flag * (shift_id - 1) / 4 * (dst[1] - src[3])`
- `opcode_sh_flag * (5 - shift_id) / 4 * (dst[0] - src[0])`
- `opcode_sh_flag * (5 - shift_id) / 4 * (dst[1] - src[1])`
- `opcode_sh_flag * (shift_id - 1) / 4 * (dst[2] - src[0])`
- `opcode_sh_flag * (shift_id - 1) / 4 * (dst[3] - src[1])`

word loads/stores

- `opcode_w_flag * (dst[i] - src[i])`

where `signed_mask = is_signed * src_msb * (2^8 - 1)` and for constraints over
i, `i` ranges over [0..3].

read instruction from the Program segment (I-type for loads and S-type for
stores)

- `- enabler * Program(pc, expected_opcode_id, rs1_addr, r2_idx, imm_felt)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_addr, rs1_clk_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3)`
- `+ enabler * Memory(REG_AS, rs1_addr, clk, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3)`
- `- RC_20(clk - rs1_clk_prev)`

check that `rs1_next_0 - shift_amount` is a multiple of 4

- `- RC_6((rs1_next_0 - shift_amount) / 2^2)`

check that rs1 is a M31

- `- RC_M31(rs1_next_0, rs1_next_3)`

read src

- `- enabler * Memory(src_as, src_addr, src_clk_prev, src_prev_0, src_prev_1, src_prev_2, src_prev_3)`
- `+ enabler * Memory(src_as, src_addr, clk, src_next_0, src_next_1, src_next_2, src_next_3)`
- `- RC_20(clk - src_clk_prev)`

write into dst

- `- enabler * Memory(dst_as, dst_addr, dst_clk_prev, dst_prev_0, dst_prev_1, dst_prev_2, dst_prev_3)`
- `+ enabler * Memory(dst_as, dst_addr, clk, dst_next_0, dst_next_1, dst_next_2, dst_next_3)`
- `- RC_20(clk - dst_clk_prev)`

## 14. MUL

- `mul`: `x[rd] = x[rs1] * x[rs2]`

### 14.1 Columns

- enabler
- pc
- clk

- rd_addr
- rd_prev_0..3
- rd_clk_prev
- rd_next_0..3

- rs1_addr
- rs1_prev_0..3
- rs1_clk_prev
- rs1_next_0..3

- rs2_addr
- rs2_prev_0..3
- rs2_clk_prev
- rs2_next_0..3

### 14.2 Variables

- `carry` from limb-wise partial sums (unused in constraints)

### 14.3 Constraints

`enabler` is a boolean

- `enabler * (1 - enabler)`

read instruction from the Program segment (R-type)

- `- enabler * Program(pc, opcode_mul_id, rd_addr, rs1_addr, rs2_addr)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_addr, rs1_clk_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3)`
- `+ enabler * Memory(REG_AS, rs1_addr, clk, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3)`
- `- RC_20(clk - rs1_clk_prev)`

read from rs2

- `- enabler * Memory(REG_AS, rs2_addr, rs2_clk_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3)`
- `+ enabler * Memory(REG_AS, rs2_addr, clk, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3)`
- `- RC_20(clk - rs2_clk_prev)`

check carries

- `- RC_8_8(carry[0], carry[1])`
- `- RC_8_8(carry[2], carry[3])`

range check rd

- `- RC_8_8(rd_next_0, rd_next_1)`
- `- RC_8_8(rd_next_2, rd_next_3)`

write to rd

- `- enabler * Memory(REG_AS, rd_addr, rd_clk_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3)`
- `+ enabler * Memory(REG_AS, rd_addr, clk, rd_next_0, rd_next_1, rd_next_2, rd_next_3)`
- `- RC_20(clk - rd_clk_prev)`

## 15. MULH (mulh/mulhsu/mulhu)

- `mulh`: `x[rd] = (x[rs1] *s x[rs2]) >> 32`
- `mulhsu`: `x[rd] = (x[rs1] *s x[rs2]) >> 32`, rs2 unsigned
- `mulhu`: `x[rd] = (x[rs1] *u x[rs2]) >> 32`

### 15.1 Columns

- pc
- clk

- rd_addr
- rd_prev_0..3
- rd_clk_prev
- rd_next_0..3

- rs1_addr
- rs1_prev_0..3
- rs1_clk_prev
- rs1_next_0..3

- rs2_addr
- rs2_prev_0..3
- rs2_clk_prev
- rs2_next_0..3

- rd_high_0
- rd_high_1
- rd_high_2
- rd_high_3

- rs1_sign
- rs2_sign

- opcode_mulh_flag
- opcode_mulhsu_flag
- opcode_mulhu_flag

### 15.2 Variables

- `enabler = opcode_mulh_flag + opcode_mulhsu_flag + opcode_mulhu_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `rs1[3] = rs1_next_3 + rs1_sign * 2^7`
- `rs2[3] = rs2_next_3 + rs2_sign * 2^7`
- `carry[0..7]` from limb-wise partial sums (unused in constraints).

### 15.3 Constraints

`enabler`, `opcode_i_flag`, and `rs*_sign` are booleans

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `rs1_sign * (1 - rs1_sign)`
- `rs2_sign * (1 - rs2_sign)`

check the signs of the operand extensions

- `(opcode_mulhsu_flag + opcode_mulhu_flag) * rs2_sign`
- `opcode_mulhu_flag * rs1_sign`

read instruction from the Program segment (R-type)

- `- enabler * Program(pc, expected_opcode_id, rd_addr, rs1_addr, rs2_addr)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_addr, rs1_clk_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3)`
- `+ enabler * Memory(REG_AS, rs1_addr, clk, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3)`
- `- RC_20(clk - rs1_clk_prev)`

read from rs2

- `- enabler * Memory(REG_AS, rs2_addr, rs2_clk_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3)`
- `+ enabler * Memory(REG_AS, rs2_addr, clk, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3)`
- `- RC_20(clk - rs2_clk_prev)`

check carries

- `- RC_8_8(carry[0], carry[1])`
- `- RC_8_8(carry[2], carry[3])`
- `- RC_8_8(carry[4], carry[5])`
- `- RC_8_8(carry[6], carry[7])`

range check rd

- `- RC_8_8(rd_next_0, rd_next_1)`
- `- RC_8_8(rd_next_2, rd_next_3)`
- `- RC_8_8(rd_high_0, rd_high_1)`
- `- RC_8_8(rd_high_2, rd_high_3)`

write to rd

- `- enabler * Memory(REG_AS, rd_addr, rd_clk_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3)`
- `+ enabler * Memory(REG_AS, rd_addr, clk, rd_next_0, rd_next_1, rd_next_2, rd_next_3)`
- `- RC_20(clk - rd_clk_prev)`

## 16. DIV (div/divu/rem/remu)

- `div`: `x[rd] = x[rs1] /s x[rs2]`
- `divu`: `x[rd] = x[rs1] /u x[rs2]`
- `rem`: `x[rd] = x[rs1] %s x[rs2]`
- `remu`: `x[rd] = x[rs1] %u x[rs2]`

### 16.1 Columns

- pc
- clk

- rd_addr
- rd_prev_0..3
- rd_clk_prev
- rd_next_0..3

- rs1_addr
- rs1_prev_0..3
- rs1_clk_prev
- rs1_next_0..3

- rs2_addr
- rs2_prev_0..3
- rs2_clk_prev
- rs2_next_0..3

- zero_divisor
- r_zero

- q_0
- q_1
- q_2
- q_3
- r_0
- r_1
- r_2
- r_3

- b_sign
- c_sign
- q_sign
- sign_xor

- c_sum_inv
- r_sum_inv

- r_abs_0
- r_abs_1
- r_abs_2
- r_abs_3

- r_inv_0
- r_inv_1
- r_inv_2
- r_inv_3

- lt_marker_0
- lt_marker_1
- lt_marker_2
- lt_marker_3
- lt_diff

- opcode_div_flag
- opcode_divu_flag
- opcode_rem_flag
- opcode_remu_flag

### 16.2 Variables

- `enabler = opcode_div_flag + opcode_divu_flag + opcode_rem_flag + opcode_remu_flag`.
- `expected_opcode_id = Σ opcode_i_flag * opcode_id_i`.
- `is_div = opcode_div_flag + opcode_divu_flag`.
- `is_signed = opcode_div_flag + opcode_rem_flag`.
- `special_case = zero_divisor + r_zero`.
- `valid_and_not_zero_divisor = enabler - zero_divisor`.
- `valid_and_not_special_case = enabler - special_case`.
- `q_sum = q_0 + q_1 + q_2 + q_3`.
- `c_sum = rs2_next_0 + rs2_next_1 + rs2_next_2 + rs2_next_3`.
- `r_sum = r_0 + r_1 + r_2 + r_3`.
- `diff[i] = (1 - 2 * c_sign) * (rs2_next[i] - r_abs[i])`.
- `carry[0..7]` from limb-wise partial sums (used by RC_8_11 lookups).
- `carry_lt[0] = (r_0 + r_abs_0) / 2^8`.
- `carry_lt[i] = (carry_lt[i - 1] + r_i + r_abs_i) / 2^8` for i in [1..3].

### 16.3 Constraints

boolean constraints

- `enabler * (1 - enabler)`
- `opcode_*_flag * (1 - opcode_*_flag)`
- `zero_divisor * (1 - zero_divisor)`
- `r_zero * (1 - r_zero)`
- `b_sign * (1 - b_sign)`
- `c_sign * (1 - c_sign)`
- `q_sign * (1 - q_sign)`
- `sign_xor * (1 - sign_xor)`
- `lt_marker_i * (1 - lt_marker_i)`
- `special_case * (1 - special_case)`
- `valid_and_not_zero_divisor * (1 - valid_and_not_zero_divisor)`
- `valid_and_not_special_case * (1 - valid_and_not_special_case)`

zero divisor detection

- `zero_divisor * rs2_next[i]`
- `zero_divisor * (q_i - (2^8 - 1))`
- `valid_and_not_zero_divisor * (c_sum * c_sum_inv - 1)`

remainder-zero detection

- `r_zero * r_i`
- `valid_and_not_special_case * (r_sum * r_sum_inv - 1)`

signed and sign xor

- `(1 - is_signed) * b_sign`
- `(1 - is_signed) * c_sign`
- `enabler * (sign_xor - b_sign - c_sign + 2 * b_sign * c_sign)`

quotient sign selection

- `(1 - zero_divisor) * q_sum * (q_sign - sign_xor)`
- `(1 - zero_divisor) * (q_sign - sign_xor) * q_sign`

absolute remainder construction

- `(1 - sign_xor) * (r_abs[i] - r_i)`
- `sign_xor * (carry_lt[i] - carry_lt[i - 1]) * (carry_lt[i] - 1)`
- `sign_xor * (1 - carry_lt[i]) * r_abs[i]`
- `sign_xor * ((r_abs[i] - 2^8) * r_inv[i] - 1)`

compare |r| with |c| from the most significant byte, `prefix_sum` starts at
`special_case`

- `enabler * (1 - prefix_sum) * diff[i]`
- `enabler * lt_marker_i * (lt_diff - diff[i])`
- `enabler * (1 - prefix_sum)`

where for constraints over i, `i` ranges over [0..3] in descending order and
`carry_lt[-1] = 0`.

read instruction from the Program segment (R-type)

- `- enabler * Program(pc, expected_opcode_id, rd_addr, rs1_addr, rs2_addr)`

registers update

- `- enabler * Registers(pc, clk)`
- `+ enabler * Registers(pc + 4, clk + 1)`

read from rs1

- `- enabler * Memory(REG_AS, rs1_addr, rs1_clk_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3)`
- `+ enabler * Memory(REG_AS, rs1_addr, clk, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3)`
- `- RC_20(clk - rs1_clk_prev)`

read from rs2

- `- enabler * Memory(REG_AS, rs2_addr, rs2_clk_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3)`
- `+ enabler * Memory(REG_AS, rs2_addr, clk, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3)`
- `- RC_20(clk - rs2_clk_prev)`

check carries to ensure that `rs1 = rs2 * q + r`

- `- enabler * RC_8_11(q_0, carry[0])`
- `- enabler * RC_8_11(q_1, carry[1])`
- `- enabler * RC_8_11(q_2, carry[2])`
- `- enabler * RC_8_11(q_3, carry[3])`
- `- enabler * RC_8_11(r_0, carry[4])`
- `- enabler * RC_8_11(r_1, carry[5])`
- `- enabler * RC_8_11(r_2, carry[6])`
- `- enabler * RC_8_11(r_3, carry[7])`

`lt_diff` is non-zero whenever the comparison is executed

- `- (enabler - special_case) * RC_20(lt_diff - 1)`

write to rd

- `- enabler * Memory(REG_AS, rd_addr, rd_clk_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3)`
- `+ enabler * Memory(REG_AS, rd_addr, clk, rd_next_0, rd_next_1, rd_next_2, rd_next_3)`
- `- RC_20(clk - rd_clk_prev)`

## 17. Preprocessed Range Check 20 (multiplicity)

The preprocessed side of the range_check_20 LogUp relation. Currently uses a
dummy constraint.

### 17.1 Columns

- multiplicity

### 17.2 Constraints

- `multiplicity - multiplicity`
