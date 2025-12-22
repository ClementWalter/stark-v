//! I/O helpers for reading guest output.
//!
//! All addresses are passed as parameters - no hardcoded constants.
//! Addresses come from linker symbols read from the ELF.

use crate::Memory;

/// Read output bytes from memory.
///
/// The output format is:
/// - `len_addr`: u32 length of the payload
/// - `data_addr`: start of payload bytes
/// - `end_addr`: end of available output region
///
/// Returns `None` if length is 0 or exceeds available space.
pub fn read_output(mem: &Memory, len_addr: u32, data_addr: u32, end_addr: u32) -> Option<Vec<u8>> {
    let len = mem.read_u32(len_addr) as usize;
    let max_len = (end_addr.saturating_sub(data_addr)) as usize;

    if len == 0 || len > max_len {
        return None;
    }

    let mut data = Vec::with_capacity(len);
    for i in 0..len {
        data.push(mem.read_u8(data_addr.wrapping_add(i as u32)));
    }

    Some(data)
}
