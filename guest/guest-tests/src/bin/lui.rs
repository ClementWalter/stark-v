//! Test for LUI (Load Upper Immediate) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            "lui t0, 0x12345",
            options(nostack, nomem)
        );
    }
    guest_tests::halt()
}
