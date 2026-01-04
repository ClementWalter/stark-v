#![no_std]
#![no_main]

use core::ptr;

unsafe extern "C" {
    static __input_start: u8;
}

fn read_input_u32() -> u32 {
    unsafe { ptr::read_volatile(ptr::addr_of!(__input_start) as *const u32) }
}

guest_bin::guest_main!({
    let n = read_input_u32();
    guest_lib::programs::fib::fib(n)
});
