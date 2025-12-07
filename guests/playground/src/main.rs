#![no_std]
#![no_main]

use core::ptr;

stark_v::entry!(main);

static mut FIB_OUTPUT: u32 = 0;

#[inline(never)]
fn main() {
    let n = 12;
    let mut prev = 0u32;
    let mut curr = 1u32;
    let mut i = 0;
    while i < n {
        let next = prev.wrapping_add(curr);
        prev = curr;
        curr = next;
        i += 1;
    }

    unsafe {
        let output = core::ptr::addr_of_mut!(FIB_OUTPUT);
        ptr::write_volatile(output, curr);
    }
}
