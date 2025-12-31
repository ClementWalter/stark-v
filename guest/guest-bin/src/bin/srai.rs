//! Test binary for SRAI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0x80000000",
            "srai t0, t1, 0",
            "srai t0, t1, 1",
            "srai t0, t1, 7",
            "srai t0, t1, 8",
            "srai t0, t1, 15",
            "srai t0, t1, 16",
            "srai t0, t1, 24",
            "srai t0, t1, 31",
            "li t1, 0x7FFFFFFF",
            "srai t0, t1, 1",
            "li t1, 0xFFFFFFFF",
            "srai t0, t1, 4",
            "li t1, 0x01234567",
            "srai t0, t1, 8",
            "li t1, 0xF0000001",
            "srai t0, t1, 4",
            "li t1, 0x80000001",
            "srai t0, t1, 31",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
