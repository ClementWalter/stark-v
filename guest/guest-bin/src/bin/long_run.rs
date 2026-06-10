//! Long-running guest: a tight arithmetic loop exceeding 10M cycles, used by
//! the segmented-recursion pipeline test.

#![no_std]
#![no_main]

guest_bin::guest_main!({
    // ~6 cycles per iteration; 1.8M iterations comfortably exceeds 10M
    // cycles while touching mul, add, xor, shift, and branch components.
    let mut acc: u32 = 0x1234_5678;
    let mut i: u32 = 0;
    while i < 1_800_000 {
        acc = acc.wrapping_mul(0x0001_0003).wrapping_add(i);
        acc ^= acc >> 7;
        i += 1;
    }
    acc
});
