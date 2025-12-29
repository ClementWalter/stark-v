//! Less Than Imm component (slti/sltiu) - airs.md Section 6

#[path = "."]
pub mod lt_imm {
    pub mod air;
    pub mod columns;
    pub mod witness;

    #[cfg(test)]
    mod tests;
}
