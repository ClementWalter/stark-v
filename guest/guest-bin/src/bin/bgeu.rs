//! Test binary for BGEU (Branch if Greater or Equal Unsigned) instruction.
//!
//! Executes the BGEU instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 1",
            "li t2, 0",
            "bgeu t1, t2, 1f",
            "nop",
            "1:",
            "li t1, 0",
            "li t2, 1",
            "bgeu t1, t2, 2f",
            "nop",
            "2:",
            "li t1, 0xFFFFFFFF",
            "li t2, 0",
            "bgeu t1, t2, 3f",
            "nop",
            "3:",
            "li t1, 0",
            "li t2, 0xFFFFFFFF",
            "bgeu t1, t2, 4f",
            "nop",
            "4:",
            "li t1, 0x80000000",
            "li t2, 0x7FFFFFFF",
            "bgeu t1, t2, 5f",
            "nop",
            "5:",
            "li t1, 0x7FFFFFFF",
            "li t2, 0x80000000",
            "bgeu t1, t2, 6f",
            "nop",
            "6:",
            "li t1, 0x00010000",
            "li t2, 0x0000FFFF",
            "bgeu t1, t2, 7f",
            "nop",
            "7:",
            "li t1, 0x0000FFFF",
            "li t2, 0x00010000",
            "bgeu t1, t2, 8f",
            "nop",
            "8:",
            "li t1, 0xFF00FF00",
            "li t2, 0x00FF00FF",
            "bgeu t1, t2, 9f",
            "nop",
            "9:",
            "li t1, 0x00FF00FF",
            "li t2, 0xFF00FF00",
            "bgeu t1, t2, 10f",
            "nop",
            "10:",
            "li t1, 0x12345678",
            "li t2, 0x12345678",
            "bgeu t1, t2, 11f",
            "nop",
            "11:",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
