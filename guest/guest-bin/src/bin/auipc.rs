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
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
