//! LogUp relations for prover components.

crate::relations! {
    program_access: addr, clk, value;
    memory_access: addr, clk, limb_0, limb_1, limb_2, limb_3;
    register_access: addr, limb_0, limb_1, limb_2, limb_3;
    range_check_20: value;
}
