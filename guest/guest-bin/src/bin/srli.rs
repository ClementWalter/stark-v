//! Test binary for SRLI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0xFF00",
            // Execute instruction 32 times
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            "srli t0, t1, 8",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
