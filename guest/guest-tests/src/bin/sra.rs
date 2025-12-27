//! Test binary for SRA instruction.
//!
//! Executes the SRA instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute SRA instruction 32 times (ensures log_size >= 5)
        // SRA: rd = rs1 >> rs2 (arithmetic shift right, sign-extend)
        asm!(
            // Load test values into registers
            "li t1, 0xF0000000",
            "li t2, 4",
            // Execute SRA 32 times
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            "sra t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_tests::halt()
}
