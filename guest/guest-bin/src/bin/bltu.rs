//! Test binary for BLTU (Branch if Less Than Unsigned) instruction.
//!
//! Executes the BLTU instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0",
            "li t2, 1",
            "bltu t1, t2, 1f",
            "nop",
            "1:",
            "li t1, 1",
            "li t2, 0",
            "bltu t1, t2, 2f",
            "nop",
            "2:",
            "li t1, 0xFFFFFFFF",
            "li t2, 0",
            "bltu t1, t2, 3f",
            "nop",
            "3:",
            "li t1, 0",
            "li t2, 0xFFFFFFFF",
            "bltu t1, t2, 4f",
            "nop",
            "4:",
            "li t1, 0x80000000",
            "li t2, 0x7FFFFFFF",
            "bltu t1, t2, 5f",
            "nop",
            "5:",
            "li t1, 0x7FFFFFFF",
            "li t2, 0x80000000",
            "bltu t1, t2, 6f",
            "nop",
            "6:",
            "li t1, 0x0000FFFF",
            "li t2, 0x00010000",
            "bltu t1, t2, 7f",
            "nop",
            "7:",
            "li t1, 0x00010000",
            "li t2, 0x0000FFFF",
            "bltu t1, t2, 8f",
            "nop",
            "8:",
            "li t1, 0x00FF00FF",
            "li t2, 0xFF00FF00",
            "bltu t1, t2, 9f",
            "nop",
            "9:",
            "li t1, 0xFF00FF00",
            "li t2, 0x00FF00FF",
            "bltu t1, t2, 10f",
            "nop",
            "10:",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
