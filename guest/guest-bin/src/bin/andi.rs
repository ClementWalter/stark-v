//! Test binary for ANDI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0xFFFFFFFF",
            "andi t0, t1, 0",
            "li t1, 0xFFFFFFFF",
            "andi t0, t1, -1",
            "li t1, 0x12345678",
            "andi t0, t1, 0x0F",
            "li t1, 0x12345678",
            "andi t0, t1, 0x7FF",
            "li t1, 0x12345678",
            "andi t0, t1, -2048",
            "li t1, 0x80000000",
            "andi t0, t1, 0x7F",
            "li t1, 0x7FFFFFFF",
            "andi t0, t1, -1",
            "li t1, 0x00FF00FF",
            "andi t0, t1, -128",
            "li t1, 0xFF00FF00",
            "andi t0, t1, 0x1F0",
            "li t1, 0x00000FFF",
            "andi t0, t1, 0x555",
            "li t1, 0xAAAAAAAA",
            "andi t0, t1, 0x555",
            "li t1, 0x55555555",
            "andi t0, t1, -512",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
