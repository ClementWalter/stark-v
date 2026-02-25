//! Minimal MUL reproducer with output.

#![no_std]
#![no_main]

use core::arch::asm;

guest_bin::guest_main!({
    let mut lo: u32;
    unsafe {
        asm!(
            "li t1, 0x00000088",
            "li t2, 0xF0F0F0F1",
            "mul t0, t1, t2",
            "mv {out_lo}, t0",
            out_lo = out(reg) lo,
            options(nostack, nomem),
        );
    }
    lo
});
