//! Test for DIV (Divide Signed) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 10",
            "li t2, 3",
            "div t0, t1, t2",
            "li t1, 10",
            "li t2, -3",
            "div t0, t1, t2",
            "li t1, -10",
            "li t2, 3",
            "div t0, t1, t2",
            "li t1, -10",
            "li t2, -3",
            "div t0, t1, t2",
            "li t1, 1",
            "li t2, 0",
            "div t0, t1, t2",
            "li t1, 0",
            "li t2, 5",
            "div t0, t1, t2",
            "li t1, 5",
            "li t2, 1",
            "div t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, -1",
            "div t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 1",
            "div t0, t1, t2",
            "li t1, 0x7FFFFFFF",
            "li t2, 2",
            "div t0, t1, t2",
            "li t1, -1",
            "li t2, 2",
            "div t0, t1, t2",
            "li t1, 123456789",
            "li t2, 1000",
            "div t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
