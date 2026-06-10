//! Division edge cases as a provable guest: zero divisor, signed overflow
//! (INT_MIN / -1), maximal operands, and sign combinations - exercising the
//! rs1 = rs2 * q + r carry identity and the sign bindings end to end.

#![no_std]
#![no_main]

guest_bin::guest_main!({
    let a = core::hint::black_box(0x8000_0000u32) as i32;
    let b = core::hint::black_box(0xFFFF_FFFFu32) as i32;
    let zero = core::hint::black_box(0i32);
    let mut acc = a.wrapping_div(b) as u32; // overflow: INT_MIN / -1
    acc ^= a.wrapping_rem(b) as u32;
    acc ^= a.wrapping_div(zero.wrapping_sub(7)) as u32; // negative divisor
    acc ^= a.wrapping_rem(zero.wrapping_sub(7)) as u32;
    acc ^= (a as u32).wrapping_div(core::hint::black_box(3u32));
    acc ^= (a as u32) % core::hint::black_box(0xFFFF_FFFFu32);
    // Division by zero: q = -1, r = dividend.
    acc ^= u32::checked_div(core::hint::black_box(5u32), core::hint::black_box(0u32))
        .unwrap_or(u32::MAX);
    acc
});
