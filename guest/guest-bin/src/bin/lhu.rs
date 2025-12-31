//! Test binary for LHU (Load Halfword Unsigned) instruction.
//!
//! Executes the LHU instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "addi t3, sp, -32",
            "li t1, 0x8001",
            "sh t1, 0(t3)",
            "li t1, 0x7FFF",
            "sh t1, 2(t3)",
            "li t1, 0x0000",
            "sh t1, 4(t3)",
            "li t1, 0xFFFF",
            "sh t1, 6(t3)",
            "lhu t0, 0(t3)",
            "lhu t0, 2(t3)",
            "lhu t0, 4(t3)",
            "lhu t0, 6(t3)",
            "lhu t0, 0(t3)",
            "lhu t0, 2(t3)",
            options(nostack)
        );
    }
    guest_bin::halt()
}
