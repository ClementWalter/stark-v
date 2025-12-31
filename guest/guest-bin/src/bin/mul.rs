//! Test for MUL (Multiply) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0",
            "li t2, 0",
            "mul t0, t1, t2",
            "li t1, 1",
            "li t2, 1",
            "mul t0, t1, t2",
            "li t1, 0",
            "li t2, -1",
            "mul t0, t1, t2",
            "li t1, -1",
            "li t2, 2",
            "mul t0, t1, t2",
            "li t1, 0x7FFFFFFF",
            "li t2, 2",
            "mul t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 2",
            "mul t0, t1, t2",
            "li t1, 0x12345678",
            "li t2, 0x9ABCDEF0",
            "mul t0, t1, t2",
            "li t1, 0x0000FFFF",
            "li t2, 0x0000FFFF",
            "mul t0, t1, t2",
            "li t1, 0x00FF00FF",
            "li t2, 0x00010001",
            "mul t0, t1, t2",
            "li t1, 0x01020304",
            "li t2, 0x05060708",
            "mul t0, t1, t2",
            "li t1, -1",
            "li t2, -1",
            "mul t0, t1, t2",
            "li t1, -1",
            "li t2, 1",
            "mul t0, t1, t2",
            "li t1, 0xFFFF0000",
            "li t2, 0x00010000",
            "mul t0, t1, t2",
            "li t1, 0x00010000",
            "li t2, 0x00010000",
            "mul t0, t1, t2",
            "li t1, 0x7FFF0000",
            "li t2, 0x0000FFFF",
            "mul t0, t1, t2",
            "li t1, 0x80000001",
            "li t2, 0x00000002",
            "mul t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
