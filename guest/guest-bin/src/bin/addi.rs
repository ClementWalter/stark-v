//! Test binary for ADDI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0x00000000",
            "addi t0, t1, 0",
            "li t1, 0x00000001",
            "addi t0, t1, 1",
            "li t1, 0x000000FF",
            "addi t0, t1, 1",
            "li t1, 0x0000FFFF",
            "addi t0, t1, 1",
            "li t1, 0x00FFFFFF",
            "addi t0, t1, 1",
            "li t1, 0xFFFFFFFF",
            "addi t0, t1, 1",
            "li t1, 0x7FFFFFFF",
            "addi t0, t1, 1",
            "li t1, 0x80000000",
            "addi t0, t1, -1",
            "li t1, 0x00000000",
            "addi t0, t1, -1",
            "li t1, 0x00000001",
            "addi t0, t1, -1",
            "li t1, 0x000007FF",
            "addi t0, t1, 0x7FF",
            "li t1, 0x00000800",
            "addi t0, t1, -2048",
            "li t1, 0x00FF00FF",
            "addi t0, t1, 0x7F",
            "li t1, 0xFF00FF00",
            "addi t0, t1, -128",
            "li t1, 0x12345678",
            "addi t0, t1, -1",
            "li t1, 0x01020304",
            "addi t0, t1, 0x123",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
