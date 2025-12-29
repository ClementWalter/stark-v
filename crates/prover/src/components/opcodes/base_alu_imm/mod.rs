//! Base ALU Imm component (addi/xori/ori/andi) - airs.md Section 2

#[path = "."]
pub mod base_alu_imm {
    pub mod air;
    pub mod columns;
    pub mod witness;

    #[cfg(test)]
    mod tests;
}
