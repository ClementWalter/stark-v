//! Test for DIV (Divide Signed) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 12345",
            "li t2, 6789",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            "div t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
