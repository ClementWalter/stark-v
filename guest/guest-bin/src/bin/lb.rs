//! Test binary for LB (Load Byte) instruction.
//!
//! Executes the LB instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "addi t3, sp, -32",
            "li t1, 0x80",
            "sb t1, 0(t3)",
            "li t1, 0x7F",
            "sb t1, 1(t3)",
            "li t1, 0x00",
            "sb t1, 2(t3)",
            "li t1, 0xFF",
            "sb t1, 3(t3)",
            "lb t0, 0(t3)",
            "lb t0, 1(t3)",
            "lb t0, 2(t3)",
            "lb t0, 3(t3)",
            "lb t0, 0(t3)",
            "lb t0, 3(t3)",
            options(nostack)
        );
    }
    guest_bin::halt()
}
