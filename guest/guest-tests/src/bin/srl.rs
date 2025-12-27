//! Test binary for SRL instruction.
//!
//! Executes the SRL instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute SRL instruction 32 times (ensures log_size >= 5)
        // SRL: rd = rs1 >> rs2 (logical shift right, zero-fill)
        asm!(
            // Load test values into registers
            "li t1, 0xF0000000",
            "li t2, 4",
            // Execute SRL 32 times
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            "srl t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_tests::halt()
}
