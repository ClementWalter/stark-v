//! Test for REMU (Remainder Unsigned) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 10",
            "li t2, 3",
            "remu t0, t1, t2",
            "li t1, 10",
            "li t2, 0",
            "remu t0, t1, t2",
            "li t1, 0",
            "li t2, 5",
            "remu t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 2",
            "remu t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 0xFFFFFFFF",
            "remu t0, t1, t2",
            "li t1, 1",
            "li t2, 0xFFFFFFFF",
            "remu t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 3",
            "remu t0, t1, t2",
            "li t1, 0x80000000",
            "li t2, 0xFFFFFFFF",
            "remu t0, t1, t2",
            "li t1, 0x00010000",
            "li t2, 0x00000010",
            "remu t0, t1, t2",
            "li t1, 0x00000010",
            "li t2, 0x00010000",
            "remu t0, t1, t2",
            "li t1, 123456789",
            "li t2, 1000",
            "remu t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
