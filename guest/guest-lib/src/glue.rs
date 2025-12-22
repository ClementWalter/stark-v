//! Shared glue code for all guest binaries.
//!
//! This module is only compiled for the riscv32 target.

use core::arch::global_asm;
use core::panic::PanicInfo;
use core::ptr;

use postcard::to_slice;
use serde::Serialize;

// -----------------------------------------------------------------------------
// Linker symbols for I/O region (defined in linker.ld)
// -----------------------------------------------------------------------------

unsafe extern "C" {
    static __halt_flag: u8;
    static __output_len: u8;
    static __output_data: u8;
    static __output_end: u8;
}

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
// Output functions
// -----------------------------------------------------------------------------

/// Serialize data with postcard and write to output region, then halt.
///
/// This function:
/// 1. Serializes `data` using postcard into the output buffer
/// 2. Writes the length to __output_len
/// 3. Sets __halt_flag to signal the host
/// 4. Loops forever (host will stop execution)
pub fn output<T: Serialize>(data: &T) -> ! {
    unsafe {
        let data_addr = ptr::addr_of!(__output_data) as *mut u8;
        let end_addr = ptr::addr_of!(__output_end) as usize;
        let data_start = data_addr as usize;
        let max_size = end_addr.saturating_sub(data_start);

        // Create a slice from the output region
        let output_buffer = core::slice::from_raw_parts_mut(data_addr, max_size);

        // Serialize with postcard
        match to_slice(data, output_buffer) {
            Ok(written) => {
                let len = written.len() as u32;
                // Write length
                let len_addr = ptr::addr_of!(__output_len) as *mut u32;
                ptr::write_volatile(len_addr, len);
            }
            Err(_) => {
                // Serialization failed - write 0 length
                let len_addr = ptr::addr_of!(__output_len) as *mut u32;
                ptr::write_volatile(len_addr, 0);
            }
        }

        // Set halt flag
        let halt_addr = ptr::addr_of!(__halt_flag) as *mut u32;
        ptr::write_volatile(halt_addr, 1);
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
