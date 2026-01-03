//! LogUp relations for prover components.
//!
//! This module generates all lookup relations and preprocessed table infrastructure.

// Use lower POW bits in debug builds to speed up tests.
#[cfg(debug_assertions)]
pub const INTERACTION_POW_BITS: u32 = 1;
#[cfg(not(debug_assertions))]
pub const INTERACTION_POW_BITS: u32 = 10;

crate::relations! {
    relations {
        registers_state: pc, clk;
        memory_access: addr_space, addr, clk, limb_0, limb_1, limb_2, limb_3;
        program_access: addr, value_0, value_1, value_2, value_3;
        merkle: index, depth, value, root;
        poseidon2: state0, state1, state2, state3, state4, state5, state6, state7,
            state8, state9, state10, state11, state12, state13, state14, state15;
    }
    preprocessed {
        bitwise: a, b, result, op_id;
        range_check_20: value;
        range_check_8_11: limb_0, limb_1;
        range_check_8_8_4: limb_0, limb_1, limb_2;
        range_check_8_8: limb_0, limb_1;
        range_check_m31: lsl, msl;
    }
}
