#![no_std]
#![no_main]

use core::{panic::PanicInfo, ptr};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
/* trunk-ignore(clippy/missing_safety_doc) */
pub unsafe extern "C" fn _start() -> ! {
    // Call main
    main();

    // Halt after main returns
    /* trunk-ignore(clippy/empty_loop) */
    loop {}
}

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

    // Prevent the optimizer from deleting the computation.
    unsafe {
        let output = core::ptr::addr_of_mut!(FIB_OUTPUT);
        ptr::write_volatile(output, curr);
    }
}
