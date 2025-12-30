//! Test binary for XORI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0xFF",
            // Execute instruction 32 times
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            "xori t0, t1, 0x0F",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
