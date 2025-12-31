//! Test for JAL (Jump And Link) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "jal t0, 1f",
            "nop",
            "1:",
            "jal x0, 2f",
            "nop",
            "2:",
            "jal t1, 3f",
            "nop",
            "3:",
            "jal t2, 4f",
            "nop",
            "4:",
            "jal x0, 6f",
            "nop",
            "5:",
            "jal t3, 7f",
            "6:",
            "jal x0, 5b",
            "7:",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
