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
    }
}
