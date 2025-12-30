//! Test binary for OR instruction.
//!
//! Executes the OR instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute OR instruction 32 times (ensures log_size >= 5)
        // OR: rd = rs1 | rs2
        asm!(
            // Load test values into registers
            "li t1, 0xF0F0F0F0",
            "li t2, 0x0F0F0F0F",
            // Execute OR 32 times
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            "or t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
