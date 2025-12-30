//! Test binary for SRAI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0xFF000000",
            // Execute instruction 32 times
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            "srai t0, t1, 8",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
