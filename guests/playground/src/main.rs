#![no_std]
#![no_main]

use core::panic::PanicInfo;

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

#[inline(never)]
fn main() {
    let x = 4;
    let y = 5;
    let _z = x + y;
}
