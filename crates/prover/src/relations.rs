//! LogUp relations for prover components.
//!
//! This module generates all lookup relations and preprocessed table infrastructure.

crate::relations! {
    relations {
        program_access: addr, clk, value;
        memory_access: addr, clk, limb_0, limb_1, limb_2, limb_3;
        register_access: addr, limb_0, limb_1, limb_2, limb_3;
    }
    preprocessed {
        range_check_20: value;
        range_check_8_8_4: limb_0, limb_1, limb_2;
        range_check_8_11: limb_0, limb_1;
        range_check_8_8: limb_0, limb_1;
        range_check_m31: lsl, msl;
        bitwise: limb_0, limb_1, result, bitwise_id;
    }
}
