//! LogUp relations for prover components.
//!
//! This module generates all lookup relations and preprocessed table infrastructure.

crate::relations! {
    relations {
        program_access: pc, opcode_id, rd_idx, rs1_idx, rs2_idx;
        registers_state: pc, clk;
        memory_access: addr_space, addr, clk, limb_0, limb_1, limb_2, limb_3;
    }
    preprocessed {
        bitwise: a, b, result, op_id;
        range_check_20: value;
        range_check_8_8_4: limb_0, limb_1, limb_2;
        range_check_8_11: limb_0, limb_1;
        range_check_8_8: limb_0, limb_1;
        range_check_m31: lsl, msl;
    }
}
