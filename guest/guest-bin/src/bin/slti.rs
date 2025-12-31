//! Test binary for SLTI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0",
            "slti t0, t1, 0",
            "li t1, 0",
            "slti t0, t1, 1",
            "li t1, 1",
            "slti t0, t1, 0",
            "li t1, -1",
            "slti t0, t1, 0",
            "li t1, -1",
            "slti t0, t1, -1",
            "li t1, -2",
            "slti t0, t1, -1",
            "li t1, 0x7FFFFFFF",
            "slti t0, t1, 0x7FF",
            "li t1, 0x7FFFFFFF",
            "slti t0, t1, -2048",
            "li t1, 0x80000000",
            "slti t0, t1, 0",
            "li t1, 0x80000000",
            "slti t0, t1, -2048",
            "li t1, 0x000007FF",
            "slti t0, t1, 0x7FF",
            "li t1, 0x000007FE",
            "slti t0, t1, 0x7FF",
            "li t1, 0x00000800",
            "slti t0, t1, -2048",
            "li t1, 0x000000FF",
            "slti t0, t1, 0x100",
            "li t1, 0x00000100",
            "slti t0, t1, 0x0FF",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
