//! Test binary for SLT instruction.
//!
//! Executes the SLT instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute SLT instruction 32 times (ensures log_size >= 5)
        // SLT: rd = (rs1 < rs2) ? 1 : 0 (signed comparison)
        asm!(
            // Load test values into registers
            // -10 in two's complement = 0xFFFFFFF6
            "li t1, 0xFFFFFFF6",
            "li t2, 10",
            // Execute SLT 32 times
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            "slt t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_tests::halt()
}
