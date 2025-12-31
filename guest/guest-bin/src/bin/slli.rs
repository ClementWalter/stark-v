//! Test binary for SLLI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0x00000001",
            "slli t0, t1, 0",
            "slli t0, t1, 1",
            "slli t0, t1, 7",
            "slli t0, t1, 8",
            "li t1, 0x01020304",
            "slli t0, t1, 9",
            "slli t0, t1, 15",
            "slli t0, t1, 16",
            "slli t0, t1, 23",
            "slli t0, t1, 24",
            "slli t0, t1, 31",
            "li t1, 0x00FF00FF",
            "slli t0, t1, 4",
            "slli t0, t1, 12",
            "li t1, 0x80000001",
            "slli t0, t1, 1",
            "slli t0, t1, 31",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
