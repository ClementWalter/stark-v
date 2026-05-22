//! Component system for tracer-backed and preprocessed AIR components.

pub mod mem_clock_update;
pub mod memory;
pub mod merkle;
pub mod opcodes;
pub mod poseidon2;
pub mod program;
pub mod reg_clock_update;

stwo_macros::components! {
    trace: {
        opcodes::auipc,
        opcodes::base_alu_imm,
        opcodes::base_alu_reg,
        opcodes::branch_eq,
        opcodes::branch_lt,
        opcodes::div,
        opcodes::jal,
        opcodes::jalr,
        opcodes::load_store,
        opcodes::lt_imm,
        opcodes::lt_reg,
        opcodes::lui,
        opcodes::mul,
        opcodes::mulh,
        opcodes::shifts_imm,
        opcodes::shifts_reg,
        program,
        memory,
        merkle,
        poseidon2,
        mem_clock_update,
        reg_clock_update,
    },
    lookup: {
        bitwise,
        range_check_20,
        range_check_8_11,
        range_check_8_8_4,
        range_check_8_8,
        range_check_m31,
    },
}

#[cfg(test)]
mod tests {
    crate::test_lookup_e2e!(base_alu_reg, bitwise, and);
    crate::test_lookup_e2e!(base_alu_reg, bitwise, or);
    crate::test_lookup_e2e!(base_alu_reg, bitwise, xor);

    crate::test_lookup_e2e!(base_alu_imm, range_check_8_8, addi);
    crate::test_lookup_e2e!(base_alu_reg, range_check_8_8, add);
    crate::test_lookup_e2e!(base_alu_reg, range_check_8_8, sub);

    crate::test_lookup_e2e!(shifts_reg, range_check_8_11, sll);
    crate::test_lookup_e2e!(shifts_reg, range_check_8_11, srl);

    crate::test_lookup_e2e!(load_store, range_check_8_8_4, lb);
    crate::test_lookup_e2e!(load_store, range_check_8_8_4, sb);

    crate::test_lookup_e2e!(div, range_check_m31, div);

    crate::test_lookup_e2e!(base_alu_reg, range_check_20, add);
    crate::test_lookup_e2e!(load_store, range_check_20, lw);
}
