//! Test binary for LH (Load Halfword) instruction.
//!
//! Executes the LH instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute LH instruction 32 times (ensures log_size >= 5)
        // LH: rd = sign_extend(mem[rs1 + imm][15:0])
        asm!(
            // Store a test value to memory first
            "li t1, 0x12345678",
            "sw t1, 0(sp)",
            // Execute LH 32 times
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            "lh t0, 0(sp)",
            options(nostack)
        );
    }
    guest_bin::halt()
}
