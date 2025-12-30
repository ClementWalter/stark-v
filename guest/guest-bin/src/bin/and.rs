//! Test binary for AND instruction.
//!
//! Executes the AND instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute AND instruction 32 times (ensures log_size >= 5)
        // AND: rd = rs1 & rs2
        asm!(
            // Load test values into registers
            "li t1, 0xFFFF0000",
            "li t2, 0xFF00FF00",
            // Execute AND 32 times
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            "and t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
