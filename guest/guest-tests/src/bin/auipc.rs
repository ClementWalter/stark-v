//! Test for AUIPC (Add Upper Immediate to PC) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            "auipc t0, 0x1",
            options(nostack, nomem)
        );
    }
    guest_tests::halt()
}
