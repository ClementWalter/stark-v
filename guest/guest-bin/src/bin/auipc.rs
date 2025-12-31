//! Test for AUIPC (Add Upper Immediate to PC) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "auipc t0, 0x00000",
            "auipc t1, 0x00001",
            "auipc t2, 0x7FFFF",
            "auipc t3, 0x80000",
            "auipc t4, 0xFFFFF",
            "auipc t5, 0x12345",
            "auipc t6, 0xABCDE",
            "auipc t0, 0x40000",
            "auipc t1, 0xC0000",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
