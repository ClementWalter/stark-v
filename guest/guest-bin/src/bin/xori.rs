//! Test binary for XORI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0x00000000",
            "xori t0, t1, 0",
            "li t1, 0x00000000",
            "xori t0, t1, -1",
            "li t1, 0x12345678",
            "xori t0, t1, 0x0F0",
            "li t1, 0x12345678",
            "xori t0, t1, -2048",
            "li t1, 0x80000000",
            "xori t0, t1, 0x7FF",
            "li t1, 0x00FF00FF",
            "xori t0, t1, 0x100",
            "li t1, 0xFF00FF00",
            "xori t0, t1, -128",
            "li t1, 0xAAAAAAAA",
            "xori t0, t1, 0x555",
            "li t1, 0x55555555",
            "xori t0, t1, -1",
            "li t1, 0x7FFFFFFF",
            "xori t0, t1, 1",
            "li t1, 0x00000FFF",
            "xori t0, t1, 0x001",
            "li t1, 0x00000001",
            "xori t0, t1, -512",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
