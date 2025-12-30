//! Test binary for SLL instruction.
//!
//! Executes the SLL instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute SLL instruction 32 times (ensures log_size >= 5)
        // SLL: rd = rs1 << rs2 (logical shift left)
        asm!(
            // Load test values into registers
            "li t1, 0x00000001",
            "li t2, 4",
            // Execute SLL 32 times
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            "sll t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
