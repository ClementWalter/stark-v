//! Test for JAL (Jump And Link) instruction

#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "jal ra, 1f",
            "1:",
            "jal ra, 2f",
            "2:",
            "jal ra, 3f",
            "3:",
            "jal ra, 4f",
            "4:",
            "jal ra, 5f",
            "5:",
            "jal ra, 6f",
            "6:",
            "jal ra, 7f",
            "7:",
            "jal ra, 8f",
            "8:",
            "jal ra, 9f",
            "9:",
            "jal ra, 10f",
            "10:",
            "jal ra, 11f",
            "11:",
            "jal ra, 12f",
            "12:",
            "jal ra, 13f",
            "13:",
            "jal ra, 14f",
            "14:",
            "jal ra, 15f",
            "15:",
            "jal ra, 16f",
            "16:",
            "jal ra, 17f",
            "17:",
            "jal ra, 18f",
            "18:",
            "jal ra, 19f",
            "19:",
            "jal ra, 20f",
            "20:",
            "jal ra, 21f",
            "21:",
            "jal ra, 22f",
            "22:",
            "jal ra, 23f",
            "23:",
            "jal ra, 24f",
            "24:",
            "jal ra, 25f",
            "25:",
            "jal ra, 26f",
            "26:",
            "jal ra, 27f",
            "27:",
            "jal ra, 28f",
            "28:",
            "jal ra, 29f",
            "29:",
            "jal ra, 30f",
            "30:",
            "jal ra, 31f",
            "31:",
            "jal ra, 32f",
            "32:",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
