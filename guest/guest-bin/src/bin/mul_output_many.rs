//! MUL-heavy program to force a larger MUL trace.

#![no_std]
#![no_main]

guest_bin::guest_main!({
    let seed = unsafe { guest_lib::io::read_input_u32() };
    // Keep rs1 confined to one byte so carry limbs stay in [0, 255].
    let a: u32 = seed & 0xFF;
    let mut b: u32 = 0xF0F0_F0F1 ^ seed.rotate_right(7);
    let mut acc: u32 = seed;
    let mut i: u32 = 0;
    while i < 64 {
        let c = a.wrapping_mul(b);
        acc ^= c.rotate_left((i & 31) + 1);
        b = b.rotate_left(3) ^ 0xA5A5_5A5A;
        i += 1;
    }
    acc
});
