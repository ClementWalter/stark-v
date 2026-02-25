//! Minimal MULHU reproducer where rd does not alias source registers.

#![no_std]
#![no_main]

use core::arch::asm;

guest_bin::guest_main!({
    let mut hi: u32;
    unsafe {
        asm!(
            "li t1, 0x00000088",
            "li t2, 0xF0F0F0F1",
            "mulhu t0, t1, t2",
            "mv {out_hi}, t0",
            out_hi = out(reg) hi,
            options(nostack, nomem),
        );
    }
    hi
});
