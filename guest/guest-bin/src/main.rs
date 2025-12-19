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
// Stack definition
// -----------------------------------------------------------------------------

#[unsafe(no_mangle)]
#[unsafe(link_section = ".data.stack")]
static __stack_top: u32 = 0x0020_0400;

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
    let _value = compute();
    loop {}
}

// -----------------------------------------------------------------------------
// Panic handler
// -----------------------------------------------------------------------------

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
