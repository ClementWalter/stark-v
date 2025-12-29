//! Base ALU Reg component (add/sub/xor/or/and) - airs.md Section 1

#[path = "."]
pub mod base_alu_reg {
    pub mod air;
    pub mod columns;
    pub mod witness;

    #[cfg(test)]
    mod tests;
}
