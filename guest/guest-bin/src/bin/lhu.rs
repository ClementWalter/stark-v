//! Test binary for LHU (Load Halfword Unsigned) instruction.
//!
//! Executes the LHU instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute LHU instruction 32 times (ensures log_size >= 5)
        // LHU: rd = zero_extend(mem[rs1 + imm][15:0])
        asm!(
            // Store a test value to memory first
            "li t1, 0x12345678",
            "sw t1, 0(sp)",
            // Execute LHU 32 times
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            "lhu t0, 0(sp)",
            options(nostack)
        );
    }
    guest_bin::halt()
}
