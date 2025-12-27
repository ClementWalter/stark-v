//! Test binary for SLTU instruction.
//!
//! Executes the SLTU instruction multiple times to generate trace data.

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        // Execute SLTU instruction 32 times (ensures log_size >= 5)
        // SLTU: rd = (rs1 < rs2) ? 1 : 0 (unsigned comparison)
        asm!(
            // Load test values into registers
            "li t1, 0xFFFFFFFF",
            "li t2, 0x00000001",
            // Execute SLTU 32 times
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            "sltu t0, t1, t2",
            options(nostack, nomem)
        );
    }
    guest_tests::halt()
}
