//! Test for MULHSU (Multiply High Signed-Unsigned) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0",
            "li t2, 0",
            "mulhsu t0, t1, t2",
            "li t1, 1",
            "li t2, 1",
            "mulhsu t0, t1, t2",
            "li t1, -1",
            "li t2, 1",
            "mulhsu t0, t1, t2",
            "li t1, -1",
            "li t2, 0xFFFFFFFF",
            "mulhsu t0, t1, t2",
            "li t1, -2",
            "li t2, 0x80000000",
            "mulhsu t0, t1, t2",
            "li t1, 0x7FFFFFFF",
            "li t2, 0xFFFFFFFF",
            "mulhsu t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 2",
            "mulhsu t0, t1, t2",
            "li t1, 0x12345678",
            "li t2, 0x9ABCDEF0",
            "mulhsu t0, t1, t2",
            "li t1, -12345",
            "li t2, 6789",
            "mulhsu t0, t1, t2",
            "li t1, 0x7FFF0000",
            "li t2, 0x0000FFFF",
            "mulhsu t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 0x80000000",
            "mulhsu t0, t1, t2",
            "li t1, 0x00010000",
            "li t2, 0x00010000",
            "mulhsu t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
