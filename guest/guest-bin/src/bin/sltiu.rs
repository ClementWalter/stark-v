//! Test binary for SLTIU instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0xFFFFFFFF",
            // Execute instruction 32 times
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            "sltiu t0, t1, 1",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
