//! Test binary for SLTI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0xFFFFFFFB", // -5 in two's complement
            // Execute instruction 32 times
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            "slti t0, t1, 0",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
