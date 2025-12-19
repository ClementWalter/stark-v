#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;
use guest_lib::compute;

// -----------------------------------------------------------------------------
// Startup assembly (ELF entrypoint)
// -----------------------------------------------------------------------------

global_asm!(
    r#"
    .section .text._start
    .globl _start
_start:
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop

    la sp, __stack_top
    lw sp, 0(sp)

    call __zkvm_start
"#
);

// -----------------------------------------------------------------------------
// Global variables
// -----------------------------------------------------------------------------

static mut INITIALIZED_COUNT: u32 = 42;
static mut ZERO_PAGE: [u8; 128] = [0; 128];

// -----------------------------------------------------------------------------
// Rust entry shim
// -----------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    main();
}

// -----------------------------------------------------------------------------
// Program logic
// -----------------------------------------------------------------------------

fn main() -> ! {
    let value = compute();
    unsafe {
        INITIALIZED_COUNT = INITIALIZED_COUNT.wrapping_add(value);
        ZERO_PAGE[0] = ZERO_PAGE[0].wrapping_add((INITIALIZED_COUNT & 0xFF) as u8);
        let sum_with_page = value.wrapping_add(ZERO_PAGE[0] as u32);
        ZERO_PAGE[1] = ZERO_PAGE[1].wrapping_add(sum_with_page as u8);
    }
    /* trunk-ignore(clippy/empty_loop) */
    loop {}
}

// -----------------------------------------------------------------------------
// Panic handler
// -----------------------------------------------------------------------------

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
