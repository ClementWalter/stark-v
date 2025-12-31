//! Test for MULH (Multiply High Signed) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0",
            "li t2, 0",
            "mulh t0, t1, t2",
            "li t1, 1",
            "li t2, 1",
            "mulh t0, t1, t2",
            "li t1, -1",
            "li t2, -1",
            "mulh t0, t1, t2",
            "li t1, -1",
            "li t2, 2",
            "mulh t0, t1, t2",
            "li t1, 2",
            "li t2, -1",
            "mulh t0, t1, t2",
            "li t1, -2",
            "li t2, -3",
            "mulh t0, t1, t2",
            "li t1, 0x7FFFFFFF",
            "li t2, 2",
            "mulh t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 2",
            "mulh t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, -1",
            "mulh t0, t1, t2",
            "li t1, 0x12345678",
            "li t2, 0x7FFFFFFF",
            "mulh t0, t1, t2",
            "li t1, 0x87654321",
            "li t2, 0x76543210",
            "mulh t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 0x7FFFFFFF",
            "mulh t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 0x80000000",
            "mulh t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
