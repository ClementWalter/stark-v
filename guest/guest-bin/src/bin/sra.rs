//! Test binary for SRA instruction.
//!
//! Executes the SRA instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0x80000000",
            "li t2, 0",
            "sra t0, t1, t2",
            "li t2, 1",
            "sra t0, t1, t2",
            "li t2, 7",
            "sra t0, t1, t2",
            "li t2, 8",
            "sra t0, t1, t2",
            "li t2, 15",
            "sra t0, t1, t2",
            "li t2, 16",
            "sra t0, t1, t2",
            "li t2, 24",
            "sra t0, t1, t2",
            "li t2, 31",
            "sra t0, t1, t2",
            "li t1, 0x7FFFFFFF",
            "li t2, 1",
            "sra t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 4",
            "sra t0, t1, t2",
            "li t1, 0x01234567",
            "li t2, 8",
            "sra t0, t1, t2",
            "li t1, 0xF0000001",
            "li t2, 4",
            "sra t0, t1, t2",
            "li t1, 0x80000001",
            "li t2, 31",
            "sra t0, t1, t2",
            "li t2, 32",
            "sra t0, t1, t2",
            "li t2, -1",
            "sra t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
