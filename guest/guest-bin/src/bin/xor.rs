//! Test binary for XOR instruction.
//!
//! Executes the XOR instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute XOR instruction 32 times (ensures log_size >= 5)
        // XOR: rd = rs1 ^ rs2
        asm!(
            // Load test values into registers
            "li t1, 0xFF00FF00",
            "li t2, 0x00FF00FF",
            // Execute XOR 32 times
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            "xor t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
