//! Test binary for SB (Store Byte) instruction.
//!
//! Executes the SB instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute SB instruction 32 times (ensures log_size >= 5)
        // SB: mem[rs1 + imm][7:0] = rs2[7:0]
        asm!(
            // Load test value into register
            "li t1, 0xFF",
            // Execute SB 32 times
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            "sb t1, 0(sp)",
            options(nostack)
        );
    }
    guest_bin::halt()
}
