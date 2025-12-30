//! Test for MULHSU (Multiply High Signed-Unsigned) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 12345",
            "li t2, 6789",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            "mulhsu t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
