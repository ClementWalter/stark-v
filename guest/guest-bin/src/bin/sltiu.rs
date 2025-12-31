//! Test binary for SLTIU instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0",
            "sltiu t0, t1, 0",
            "li t1, 0",
            "sltiu t0, t1, 1",
            "li t1, 1",
            "sltiu t0, t1, 0",
            "li t1, 0xFFFFFFFF",
            "sltiu t0, t1, -1",
            "li t1, 0xFFFFFFFE",
            "sltiu t0, t1, -1",
            "li t1, 0xFFFFFFFF",
            "sltiu t0, t1, 0x7FF",
            "li t1, 0x000007FF",
            "sltiu t0, t1, 0x7FF",
            "li t1, 0x000007FE",
            "sltiu t0, t1, 0x7FF",
            "li t1, 0x80000000",
            "sltiu t0, t1, -2048",
            "li t1, 0xFFFFF800",
            "sltiu t0, t1, -2048",
            "li t1, 0xFFFFF801",
            "sltiu t0, t1, -2048",
            "li t1, 0x000000FF",
            "sltiu t0, t1, 0x100",
            "li t1, 0x00000100",
            "sltiu t0, t1, 0x0FF",
            "li t1, 0x00001000",
            "sltiu t0, t1, -2048",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
