//! Test for DIVU (Divide Unsigned) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 10",
            "li t2, 3",
            "divu t0, t1, t2",
            "li t1, 0",
            "li t2, 5",
            "divu t0, t1, t2",
            "li t1, 5",
            "li t2, 0",
            "divu t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 2",
            "divu t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 2",
            "divu t0, t1, t2",
            "li t1, 1",
            "li t2, 0xFFFFFFFF",
            "divu t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 0xFFFFFFFF",
            "divu t0, t1, t2",
            "li t1, 0x00010000",
            "li t2, 0x00000010",
            "divu t0, t1, t2",
            "li t1, 0x00000010",
            "li t2, 0x00010000",
            "divu t0, t1, t2",
            "li t1, 0xFFFFFFFE",
            "li t2, 0x00000002",
            "divu t0, t1, t2",
            "li t1, 123456789",
            "li t2, 1000",
            "divu t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 0xFFFFFFFF",
            "divu t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
