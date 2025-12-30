//! Test binary for SUB instruction.
//!
//! Executes the SUB instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute SUB instruction 32 times (ensures log_size >= 5)
        // SUB: rd = rs1 - rs2
        asm!(
            // Load test values into registers
            "li t1, 0xFFFFFFFF",
            "li t2, 0x00000001",
            // Execute SUB 32 times
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            "sub t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
