//! Test binary for SLTU instruction.
//!
//! Executes the SLTU instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0",
            "li t2, 0",
            "sltu t0, t1, t2",
            "li t1, 0",
            "li t2, 1",
            "sltu t0, t1, t2",
            "li t1, 1",
            "li t2, 0",
            "sltu t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 0",
            "sltu t0, t1, t2",
            "li t1, 0",
            "li t2, 0xFFFFFFFF",
            "sltu t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 0x7FFFFFFF",
            "sltu t0, t1, t2",
            "li t1, 0x7FFFFFFF",
            "li t2, 0x80000000",
            "sltu t0, t1, t2",
            "li t1, 0x0000FFFF",
            "li t2, 0x00010000",
            "sltu t0, t1, t2",
            "li t1, 0x00010000",
            "li t2, 0x0000FFFF",
            "sltu t0, t1, t2",
            "li t1, 0x00FF00FF",
            "li t2, 0xFF00FF00",
            "sltu t0, t1, t2",
            "li t1, 0xFF00FF00",
            "li t2, 0x00FF00FF",
            "sltu t0, t1, t2",
            "li t1, 0x000000FF",
            "li t2, 0x00000100",
            "sltu t0, t1, t2",
            "li t1, 0x00000100",
            "li t2, 0x000000FF",
            "sltu t0, t1, t2",
            "li t1, 0x12345678",
            "li t2, 0x12345678",
            "sltu t0, t1, t2",
            "li t1, 0x01000000",
            "li t2, 0x02000000",
            "sltu t0, t1, t2",
            "li t1, 0x02000000",
            "li t2, 0x01000000",
            "sltu t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
