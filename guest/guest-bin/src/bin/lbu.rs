//! Test binary for LBU (Load Byte Unsigned) instruction.
//!
//! Executes the LBU instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute LBU instruction 32 times (ensures log_size >= 5)
        // LBU: rd = zero_extend(mem[rs1 + imm][7:0])
        asm!(
            // Store a test value to memory first
            "li t1, 0x12345678",
            "sw t1, 0(sp)",
            // Execute LBU 32 times
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            "lbu t0, 0(sp)",
            options(nostack)
        );
    }
    guest_bin::halt()
}
