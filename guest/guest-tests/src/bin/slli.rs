//! Test binary for SLLI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 1",
            // Execute instruction 32 times
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            "slli t0, t1, 8",
            options(nostack, nomem)
        );
    }
    guest_tests::halt()
}
