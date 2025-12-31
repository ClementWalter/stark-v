//! Test binary for SLT instruction.
//!
//! Executes the SLT instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, -1",
            "li t2, 1",
            "slt t0, t1, t2",
            "li t1, 1",
            "li t2, -1",
            "slt t0, t1, t2",
            "li t1, -2",
            "li t2, -1",
            "slt t0, t1, t2",
            "li t1, -1",
            "li t2, -2",
            "slt t0, t1, t2",
            "li t1, 0",
            "li t2, 0",
            "slt t0, t1, t2",
            "li t1, 0",
            "li t2, 1",
            "slt t0, t1, t2",
            "li t1, 1",
            "li t2, 0",
            "slt t0, t1, t2",
            "li t1, 0x7FFFFFFF",
            "li t2, 0x80000000",
            "slt t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 0",
            "slt t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 0x7FFFFFFF",
            "slt t0, t1, t2",
            "li t1, 0x00010000",
            "li t2, 0x00020000",
            "slt t0, t1, t2",
            "li t1, 0x01000000",
            "li t2, 0x02000000",
            "slt t0, t1, t2",
            "li t1, -1",
            "li t2, -1",
            "slt t0, t1, t2",
            "li t1, 0x000000FF",
            "li t2, 0x00000100",
            "slt t0, t1, t2",
            "li t1, 0x00000100",
            "li t2, 0x000000FF",
            "slt t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 0xFFFFFFFF",
            "slt t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
