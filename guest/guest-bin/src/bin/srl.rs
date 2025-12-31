//! Test binary for SRL instruction.
//!
//! Executes the SRL instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0x80000000",
            "li t2, 0",
            "srl t0, t1, t2",
            "li t2, 1",
            "srl t0, t1, t2",
            "li t1, 0xFFFFFFFF",
            "li t2, 4",
            "srl t0, t1, t2",
            "li t1, 0x01234567",
            "li t2, 8",
            "srl t0, t1, t2",
            "li t2, 16",
            "srl t0, t1, t2",
            "li t2, 24",
            "srl t0, t1, t2",
            "li t1, 0x89ABCDEF",
            "li t2, 7",
            "srl t0, t1, t2",
            "li t1, 0x00000001",
            "li t2, 1",
            "srl t0, t1, t2",
            "li t1, 0x80000001",
            "li t2, 31",
            "srl t0, t1, t2",
            "li t1, 0x7FFFFFFF",
            "li t2, 31",
            "srl t0, t1, t2",
            "li t1, 0x01020304",
            "li t2, 15",
            "srl t0, t1, t2",
            "li t1, 0x00FF00FF",
            "li t2, 9",
            "srl t0, t1, t2",
            "li t1, 0x00000001",
            "li t2, 32",
            "srl t0, t1, t2",
            "li t2, -1",
            "srl t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
