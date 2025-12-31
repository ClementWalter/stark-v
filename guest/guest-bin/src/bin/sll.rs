//! Test binary for SLL instruction.
//!
//! Executes the SLL instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0x00000001",
            "li t2, 0",
            "sll t0, t1, t2",
            "li t2, 1",
            "sll t0, t1, t2",
            "li t2, 7",
            "sll t0, t1, t2",
            "li t2, 8",
            "sll t0, t1, t2",
            "li t2, 15",
            "sll t0, t1, t2",
            "li t2, 16",
            "sll t0, t1, t2",
            "li t2, 24",
            "sll t0, t1, t2",
            "li t2, 31",
            "sll t0, t1, t2",
            "li t1, 0x01020304",
            "li t2, 9",
            "sll t0, t1, t2",
            "li t1, 0x00FF00FF",
            "li t2, 4",
            "sll t0, t1, t2",
            "li t1, 0x80000001",
            "li t2, 1",
            "sll t0, t1, t2",
            "li t1, 0x00000001",
            "li t2, 32",
            "sll t0, t1, t2",
            "li t2, 63",
            "sll t0, t1, t2",
            "li t2, -1",
            "sll t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
