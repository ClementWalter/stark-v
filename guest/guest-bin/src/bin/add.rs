//! Test binary for ADD instruction.
//!
//! Executes the ADD instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            // Carry/overflow coverage across limbs
            "li t1, 0x00000000",
            "li t2, 0x00000000",
            "add t0, t1, t2",
            "li t1, 0x00000001",
            "li t2, 0x00000001",
            "add t0, t1, t2",
            "li t1, 0x000000FF",
            "li t2, 0x00000001",
            "add t0, t1, t2",
            "li t1, 0x0000FFFF",
            "li t2, 0x00000001",
            "add t0, t1, t2",
            "li t1, 0x00FFFFFF",
            "li t2, 0x00000001",
            "add t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 0x00000001",
            "add t0, t1, t2",
            "li t1, 0x7FFFFFFF",
            "li t2, 0x00000001",
            "add t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 0x80000000",
            "add t0, t1, t2",
            "li t1, 0x01020304",
            "li t2, 0x05060708",
            "add t0, t1, t2",
            "li t1, 0xFEFEFEFE",
            "li t2, 0x01010101",
            "add t0, t1, t2",
            "li t1, 0x12345678",
            "li t2, 0x87654321",
            "add t0, t1, t2",
            "li t1, 0x00FF00FF",
            "li t2, 0x00010001",
            "add t0, t1, t2",
            "li t1, 0x00FF00FF",
            "li t2, 0x00FF00FF",
            "add t0, t1, t2",
            "li t1, 0xFF00FF00",
            "li t2, 0x01000100",
            "add t0, t1, t2",
            "li t1, 0x7F7F7F7F",
            "li t2, 0x01010101",
            "add t0, t1, t2",
            "li t1, 0x80808080",
            "li t2, 0x80808080",
            "add t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
