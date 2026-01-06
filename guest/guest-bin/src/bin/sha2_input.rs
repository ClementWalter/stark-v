//! SHA256 with variable-length input - demonstrates using guest_lib::io for byte input.
//!
//! Input format: first 4 bytes are length (u32 little-endian), followed by the message bytes.

#![no_std]
#![no_main]

guest_bin::guest_main!({
    // Input format: [len: u32][data: u8...]
    // First read the length prefix (4 bytes)
    let len = unsafe { guest_lib::io::read_input_u32() } as usize;

    // Read only the exact number of data bytes needed (up to 1020 to fit in buffer)
    let data_len = len.min(1020);
    let mut buf = [0u8; 1024];

    // Read data bytes starting at offset 4 (after the length prefix)
    if data_len > 0 {
        unsafe { guest_lib::io::read_input_bytes_at(4, &mut buf[..data_len]) };
    }

    guest_lib::programs::sha2::sha256(&buf[..data_len])
});
