//! Test binary for SH (Store Halfword) instruction.
//!
//! Executes the SH instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute SH instruction 32 times (ensures log_size >= 5)
        // SH: mem[rs1 + imm][15:0] = rs2[15:0]
        asm!(
            // Load test value into register
            "li t1, 0xFFFF",
            // Execute SH 32 times
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            "sh t1, 0(sp)",
            options(nostack)
        );
    }
    guest_tests::halt()
}
