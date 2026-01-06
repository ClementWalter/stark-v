//! SHA256 with variable-length input - demonstrates using guest_lib::io for byte input.
//!
//! Input format: first 4 bytes are length (u32 little-endian), followed by the message bytes.

#![no_std]
#![no_main]

guest_bin::guest_main!({
    // Read the full input into a buffer
    // Input format: [len: u32][data: u8...]
    let mut buf = [0u8; 1024]; // Max 1KB input
    let bytes_read = unsafe { guest_lib::io::read_input_bytes(&mut buf) };

    // First 4 bytes are the length
    let len = if bytes_read >= 4 {
        u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize
    } else {
        0
    };

    // Compute SHA256 on the data portion (after the 4-byte length prefix)
    let data_start = 4;
    let data_end = (data_start + len).min(bytes_read);
    let data = &buf[data_start..data_end];

    guest_lib::programs::sha2::sha256(data)
});
