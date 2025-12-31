//! Test binary for SH (Store Halfword) instruction.
//!
//! Executes the SH instruction multiple times to generate trace data.

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
            "sh t1, 0(t3)",
            "li t1, 0xFFFF",
            "sh t1, 2(t3)",
            "li t1, 0x1234",
            "sh t1, 0(t3)",
            "li t1, 0xABCD",
            "sh t1, 2(t3)",
            options(nostack)
        );
    }
    guest_bin::halt()
}
