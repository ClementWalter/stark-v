//! Test binary for SRLI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0x80000000",
            "srli t0, t1, 0",
            "srli t0, t1, 1",
            "li t1, 0xFFFFFFFF",
            "srli t0, t1, 4",
            "li t1, 0x01234567",
            "srli t0, t1, 8",
            "srli t0, t1, 16",
            "srli t0, t1, 24",
            "li t1, 0x89ABCDEF",
            "srli t0, t1, 7",
            "li t1, 0x00000001",
            "srli t0, t1, 1",
            "li t1, 0x80000001",
            "srli t0, t1, 31",
            "li t1, 0x7FFFFFFF",
            "srli t0, t1, 31",
            "li t1, 0x01020304",
            "srli t0, t1, 15",
            "li t1, 0x00FF00FF",
            "srli t0, t1, 9",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
