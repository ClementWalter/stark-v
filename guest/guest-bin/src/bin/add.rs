//! Test binary for ADD instruction.
//!
//! Executes the ADD instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute ADD instruction 32 times (ensures log_size >= 5)
        // ADD: rd = rs1 + rs2
        asm!(
            // Load test values into registers
            "li t1, 0x12345678",
            "li t2, 0x87654321",
            // Execute ADD 32 times
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            "add t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
