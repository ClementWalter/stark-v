//! SHA256 with variable-length input - demonstrates using guest_lib::io for byte input.
//!
//! Input format: first 4 bytes are length (u32 little-endian), followed by the message bytes.

#![no_std]
#![no_main]

// Keep the message buffer off the stack; stack is 1 KiB in the linker script.
// Input buffer is 4 KiB in linker.ld; reserve 4 bytes for the length prefix.
const INPUT_CAPACITY: usize = 4096;
const MAX_MSG_LEN: usize = INPUT_CAPACITY - 4;
static mut INPUT_BUF: [u8; MAX_MSG_LEN] = [0u8; MAX_MSG_LEN];

guest_bin::guest_main!({
    // Input format: [len: u32][data: u8...]
    // First read the length prefix (4 bytes)
    let len = unsafe { guest_lib::io::read_input_u32() } as usize;

    let data_len = len.min(MAX_MSG_LEN);
    let buf = unsafe {
        core::slice::from_raw_parts_mut(core::ptr::addr_of_mut!(INPUT_BUF) as *mut u8, MAX_MSG_LEN)
    };

    // Read data bytes starting at offset 4 (after the length prefix).
    let read_len = unsafe { guest_lib::io::read_input_bytes_at(4, &mut buf[..data_len]) };

    guest_lib::programs::sha2::sha256(&buf[..read_len])
});
