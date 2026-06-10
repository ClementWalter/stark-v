//! Multiplication with maximal operands: the 0xFFFFFFFF * 0xFFFFFFFF carry
//! chain reaches the widest honest carries (above 2^8), exercising the
//! mul/mulh carry range checks at their bound.

#![no_std]
#![no_main]

guest_bin::guest_main!({
    let a: u32 = core::hint::black_box(0xFFFF_FFFF);
    let b: u32 = core::hint::black_box(0xFFFF_FFFF);
    let low = a.wrapping_mul(b);
    let high = (((a as u64) * (b as u64)) >> 32) as u32;
    low ^ high
});
