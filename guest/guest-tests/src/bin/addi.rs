//! Test binary for ADDI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 100",
            // Execute instruction 32 times
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            "addi t0, t1, 100",
            options(nostack, nomem)
        );
    }
    guest_tests::halt()
}
