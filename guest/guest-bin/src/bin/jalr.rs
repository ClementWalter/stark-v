//! Test for JALR (Jump And Link Register) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "la t1, 1f",
            "jalr t0, t1, 0",
            "nop",
            "1:",
            "la t2, 2f",
            "jalr t3, t2, 1",
            "nop",
            "2:",
            "la t4, 3f",
            "addi t4, t4, 1",
            "jalr t5, t4, 0",
            "nop",
            "3:",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
