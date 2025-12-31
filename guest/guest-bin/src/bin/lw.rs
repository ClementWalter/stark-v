//! Test binary for LW (Load Word) instruction.
//!
//! Executes the LW instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "addi t3, sp, -32",
            "li t1, 0x12345678",
            "sw t1, 0(t3)",
            "li t1, 0x89ABCDEF",
            "sw t1, 4(t3)",
            "li t1, 0xFFFFFFFF",
            "sw t1, 8(t3)",
            "li t1, 0x00000000",
            "sw t1, 12(t3)",
            "lw t0, 0(t3)",
            "lw t0, 4(t3)",
            "lw t0, 8(t3)",
            "lw t0, 12(t3)",
            "lw t0, 0(t3)",
            "lw t0, 4(t3)",
            options(nostack)
        );
    }
    guest_bin::halt()
}
