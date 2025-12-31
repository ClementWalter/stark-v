//! Test for LUI (Load Upper Immediate) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "lui t0, 0x00000",
            "lui t0, 0x00001",
            "lui t0, 0x7FFFF",
            "lui t0, 0x80000",
            "lui t0, 0xFFFFF",
            "lui t0, 0x12345",
            "lui t0, 0xABCDE",
            "lui t0, 0x40000",
            "lui t0, 0xC0000",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
