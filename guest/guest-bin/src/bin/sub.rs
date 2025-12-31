//! Test binary for SUB instruction.
//!
//! Executes the SUB instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            // Borrow propagation coverage
            "li t1, 0x00000000",
            "li t2, 0x00000000",
            "sub t0, t1, t2",
            "li t1, 0x00000001",
            "li t2, 0x00000001",
            "sub t0, t1, t2",
            "li t1, 0x00000000",
            "li t2, 0x00000001",
            "sub t0, t1, t2",
            "li t1, 0x00010000",
            "li t2, 0x00000001",
            "sub t0, t1, t2",
            "li t1, 0x01000000",
            "li t2, 0x00000001",
            "sub t0, t1, t2",
            "li t1, 0x10000000",
            "li t2, 0x00000001",
            "sub t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 0x00000001",
            "sub t0, t1, t2",
            "li t1, 0x7FFFFFFF",
            "li t2, 0xFFFFFFFF",
            "sub t0, t1, t2",
            "li t1, 0x00000000",
            "li t2, 0xFFFFFFFF",
            "sub t0, t1, t2",
            "li t1, 0x12345678",
            "li t2, 0x01020304",
            "sub t0, t1, t2",
            "li t1, 0x00FF0000",
            "li t2, 0x00010000",
            "sub t0, t1, t2",
            "li t1, 0xFF00FF00",
            "li t2, 0x00FF00FF",
            "sub t0, t1, t2",
            "li t1, 0x00000080",
            "li t2, 0x0000007F",
            "sub t0, t1, t2",
            "li t1, 0x00000080",
            "li t2, 0x00000081",
            "sub t0, t1, t2",
            "li t1, 0x01020304",
            "li t2, 0x05060708",
            "sub t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 0xFFFFFFFF",
            "sub t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
