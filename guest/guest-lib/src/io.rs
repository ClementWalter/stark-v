//! I/O memory layout and helper functions for guest programs.
//!
//! These constants match the memory layout defined in `guest-bin/linker.ld`.
//! External consumers can use these to understand the zkVM's memory interface.

/// Start address of the input buffer (4 KiB).
pub const INPUT_START: u32 = 0x0010_0000;

/// End address of the input buffer (exclusive).
pub const INPUT_END: u32 = 0x0010_1000;

/// Input buffer size in bytes.
pub const INPUT_SIZE: usize = (INPUT_END - INPUT_START) as usize;

/// Address of the halt flag (set to non-zero to halt execution).
pub const HALT_FLAG: u32 = 0x0010_1000;

/// Address of the output length word.
pub const OUTPUT_LEN: u32 = 0x0010_1004;

/// Start address of the output data buffer.
pub const OUTPUT_DATA: u32 = 0x0010_1008;

/// Stack top address.
pub const STACK_TOP: u32 = 0x0020_0000;

/// Stack size in bytes (1 KiB).
pub const STACK_SIZE: usize = 0x0000_0400;

/// Stack bottom address.
pub const STACK_BOTTOM: u32 = STACK_TOP - STACK_SIZE as u32;

/// End address of the output data buffer (exclusive, equals stack bottom).
pub const OUTPUT_END: u32 = STACK_BOTTOM;

/// Maximum output size in bytes.
pub const OUTPUT_MAX_SIZE: usize = (OUTPUT_END - OUTPUT_DATA) as usize;

/// Read input bytes from the input buffer.
///
/// # Safety
/// This function reads from raw memory addresses. Only call from within
/// a zkVM guest program with the correct memory layout.
#[cfg(target_arch = "riscv32")]
pub unsafe fn read_input_bytes(buf: &mut [u8]) -> usize {
    unsafe {
        let len = buf.len().min(INPUT_SIZE);
        for (i, byte) in buf.iter_mut().take(len).enumerate() {
            let addr = INPUT_START + i as u32;
            *byte = core::ptr::read_volatile(addr as *const u8);
        }
        len
    }
}

/// Write output bytes to the output buffer and set the length.
///
/// # Safety
/// This function writes to raw memory addresses. Only call from within
/// a zkVM guest program with the correct memory layout.
#[cfg(target_arch = "riscv32")]
pub unsafe fn write_output_bytes(data: &[u8]) {
    unsafe {
        let len = data.len().min(OUTPUT_MAX_SIZE);
        // Write length
        core::ptr::write_volatile(OUTPUT_LEN as *mut u32, len as u32);
        // Write data
        for (i, byte) in data.iter().take(len).enumerate() {
            let addr = OUTPUT_DATA + i as u32;
            core::ptr::write_volatile(addr as *mut u8, *byte);
        }
    }
}

/// Signal halt to the zkVM runtime.
///
/// # Safety
/// This function writes to raw memory addresses. Only call from within
/// a zkVM guest program with the correct memory layout.
#[cfg(target_arch = "riscv32")]
pub unsafe fn halt() {
    unsafe {
        core::ptr::write_volatile(HALT_FLAG as *mut u32, 1);
    }
}
