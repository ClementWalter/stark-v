//! Test binary for AND instruction.
//!
//! Executes the AND instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0xFFFFFFFF",
            "li t2, 0x00000000",
            "and t0, t1, t2",
            "li t1, 0xAAAAAAAA",
            "li t2, 0x55555555",
            "and t0, t1, t2",
            "li t1, 0xFFFF0000",
            "li t2, 0x00FFFF00",
            "and t0, t1, t2",
            "li t1, 0x12345678",
            "li t2, 0x87654321",
            "and t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 0x7FFFFFFF",
            "and t0, t1, t2",
            "li t1, 0x00FF00FF",
            "li t2, 0xFF00FF00",
            "and t0, t1, t2",
            "li t1, 0x0F0F0F0F",
            "li t2, 0xF0F0F0F0",
            "and t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 0xFFFFFFFF",
            "and t0, t1, t2",
            "li t1, 0x0000FFFF",
            "li t2, 0x00FF00FF",
            "and t0, t1, t2",
            "li t1, 0x01020304",
            "li t2, 0xFFFFFFFF",
            "and t0, t1, t2",
            "li t1, 0x7F7F7F7F",
            "li t2, 0x80808080",
            "and t0, t1, t2",
            "li t1, 0x13579BDF",
            "li t2, 0x02468ACE",
            "and t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
