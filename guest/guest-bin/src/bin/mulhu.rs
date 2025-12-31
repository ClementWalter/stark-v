//! Test for MULHU (Multiply High Unsigned) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0",
            "li t2, 0",
            "mulhu t0, t1, t2",
            "li t1, 1",
            "li t2, 1",
            "mulhu t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 0xFFFFFFFF",
            "mulhu t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 2",
            "mulhu t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 2",
            "mulhu t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 0x80000000",
            "mulhu t0, t1, t2",
            "li t1, 0x12345678",
            "li t2, 0x9ABCDEF0",
            "mulhu t0, t1, t2",
            "li t1, 0x0000FFFF",
            "li t2, 0x0000FFFF",
            "mulhu t0, t1, t2",
            "li t1, 0x00FF00FF",
            "li t2, 0x00010001",
            "mulhu t0, t1, t2",
            "li t1, 0x00010000",
            "li t2, 0x00010000",
            "mulhu t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 0x00000001",
            "mulhu t0, t1, t2",
            "li t1, 0x7FFFFFFF",
            "li t2, 0x7FFFFFFF",
            "mulhu t0, t1, t2",
            "li t1, 0xFFFF0000",
            "li t2, 0x00010000",
            "mulhu t0, t1, t2",
            "li t1, 0xFFFFFFFE",
            "li t2, 0x00000002",
            "mulhu t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
