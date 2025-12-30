//! Test binary for SW (Store Word) instruction.
//!
//! Executes the SW instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute SW instruction 32 times (ensures log_size >= 5)
        // SW: mem[rs1 + imm][31:0] = rs2[31:0]
        asm!(
            // Load test value into register
            "li t1, 0x12345678",
            // Execute SW 32 times
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            "sw t1, 0(sp)",
            options(nostack)
        );
    }
    guest_bin::halt()
}
