//! Shared glue code for all guest binaries.
//!
//! This module is only compiled for the riscv32 target.

use core::arch::global_asm;
use core::panic::PanicInfo;
use core::ptr;

use guest_lib::io::{HALT_FLAG, OUTPUT_DATA, OUTPUT_END, OUTPUT_LEN};
use postcard::to_slice;
use serde::Serialize;

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
// Halt function (for opcode tests that don't produce output)
// -----------------------------------------------------------------------------

/// Halt the VM by spinning forever.
/// The runner will detect the PC not changing and stop execution.
/// Used by opcode test binaries that only need to generate traces.
#[inline(never)]
pub fn halt() -> ! {
    #[allow(clippy::empty_loop)]
    loop {}
}

// -----------------------------------------------------------------------------
// Output functions
// -----------------------------------------------------------------------------

/// Serialize data with postcard and write to output region, then halt.
///
/// This function:
/// 1. Serializes `data` using postcard into the output buffer
/// 2. Writes the length to OUTPUT_LEN
/// 3. Sets HALT_FLAG to signal the host
/// 4. Loops forever (host will stop execution)
pub fn output<T: Serialize>(data: &T) -> ! {
    unsafe {
        let max_size = (OUTPUT_END - OUTPUT_DATA) as usize;

        // Create a slice from the output region
        let output_buffer = core::slice::from_raw_parts_mut(OUTPUT_DATA as *mut u8, max_size);

        // Serialize with postcard
        match to_slice(data, output_buffer) {
            Ok(written) => {
                let len = written.len() as u32;
                // Write length
                ptr::write_volatile(OUTPUT_LEN as *mut u32, len);
            }
            Err(_) => {
                // Serialization failed - write 0 length
                ptr::write_volatile(OUTPUT_LEN as *mut u32, 0);
            }
        }

        // Set halt flag
        ptr::write_volatile(HALT_FLAG as *mut u32, 1);
    }

    // Should never reach here - host stops on halt flag
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
