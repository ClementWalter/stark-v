//! Test binary for OR instruction.
//!
//! Executes the OR instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0x00000000",
            "li t2, 0x00000000",
            "or t0, t1, t2",
            "li t1, 0xAAAAAAAA",
            "li t2, 0x55555555",
            "or t0, t1, t2",
            "li t1, 0xFFFF0000",
            "li t2, 0x00FFFF00",
            "or t0, t1, t2",
            "li t1, 0x12345678",
            "li t2, 0x87654321",
            "or t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 0x7FFFFFFF",
            "or t0, t1, t2",
            "li t1, 0x00FF00FF",
            "li t2, 0xFF00FF00",
            "or t0, t1, t2",
            "li t1, 0x0F0F0F0F",
            "li t2, 0xF0F0F0F0",
            "or t0, t1, t2",
            "li t1, 0x0000FFFF",
            "li t2, 0x00FF00FF",
            "or t0, t1, t2",
            "li t1, 0x01020304",
            "li t2, 0x00000000",
            "or t0, t1, t2",
            "li t1, 0x13579BDF",
            "li t2, 0x02468ACE",
            "or t0, t1, t2",
            "li t1, 0x7F7F7F7F",
            "li t2, 0x80808080",
            "or t0, t1, t2",
            "li t1, 0xF0F0F0F0",
            "li t2, 0x0F0F0F0F",
            "or t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
