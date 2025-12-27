//! Test binary for LB (Load Byte) instruction.
//!
//! Executes the LB instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute LB instruction 32 times (ensures log_size >= 5)
        // LB: rd = sign_extend(mem[rs1 + imm][7:0])
        asm!(
            // Store a test value to memory first
            "li t1, 0x12345678",
            "sw t1, 0(sp)",
            // Execute LB 32 times
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            "lb t0, 0(sp)",
            options(nostack)
        );
    }
    guest_tests::halt()
}
