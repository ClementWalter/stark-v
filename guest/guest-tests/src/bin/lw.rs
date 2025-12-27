//! Test binary for LW (Load Word) instruction.
//!
//! Executes the LW instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute LW instruction 32 times (ensures log_size >= 5)
        // LW: rd = mem[rs1 + imm][31:0]
        asm!(
            // Store a test value to memory first
            "li t1, 0x12345678",
            "sw t1, 0(sp)",
            // Execute LW 32 times
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            "lw t0, 0(sp)",
            options(nostack)
        );
    }
    guest_tests::halt()
}
