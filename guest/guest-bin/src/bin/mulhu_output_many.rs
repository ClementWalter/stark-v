//! MULHU-heavy program to force a larger MULH trace.

#![no_std]
#![no_main]

use core::arch::asm;

guest_bin::guest_main!({
    let seed = unsafe { guest_lib::io::read_input_u32() };
    let a: u32 = seed & 0xFF;
    let mut b: u32 = 0xF0F0_F0F1 ^ seed.rotate_right(7);
    let mut acc: u32 = seed;
    let mut i: u32 = 0;
    while i < 64 {
        let mut hi: u32;
        unsafe {
            asm!(
                "mulhu {out_hi}, {lhs}, {rhs}",
                out_hi = out(reg) hi,
                lhs = in(reg) a,
                rhs = in(reg) b,
                options(nostack, nomem),
            );
        }
        acc ^= hi.rotate_left((i & 31) + 1);
        b = b.rotate_left(3) ^ 0xA5A5_5A5A;
        i += 1;
    }
    acc
});
