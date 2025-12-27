//! Minimal glue code for opcode test binaries.
//!
//! Each test binary executes a specific RISC-V instruction multiple times
//! to generate trace data for AIR constraint testing.

#![no_std]

use core::arch::global_asm;
use core::panic::PanicInfo;

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

    call __zkvm_start
"#
);

// -----------------------------------------------------------------------------
// Halt function
// -----------------------------------------------------------------------------

/// Halt the VM by spinning forever.
/// The runner will detect the PC not changing and stop execution.
#[inline(never)]
pub fn halt() -> ! {
    #[allow(clippy::empty_loop)]
    loop {}
}

// -----------------------------------------------------------------------------
// Panic handler
// -----------------------------------------------------------------------------

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    #[allow(clippy::empty_loop)]
    loop {}
}
