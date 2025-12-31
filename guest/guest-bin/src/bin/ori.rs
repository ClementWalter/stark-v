//! Test binary for ORI instruction.
#![no_std]
#![no_main]
use core::arch::asm;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    unsafe {
        asm!(
            "li t1, 0x00000000",
            "ori t0, t1, 0",
            "li t1, 0x00000000",
            "ori t0, t1, -1",
            "li t1, 0x12345678",
            "ori t0, t1, 0x0F0",
            "li t1, 0x12345678",
            "ori t0, t1, -2048",
            "li t1, 0x80000000",
            "ori t0, t1, 0x7FF",
            "li t1, 0x00FF00FF",
            "ori t0, t1, 0x100",
            "li t1, 0xFF00FF00",
            "ori t0, t1, -128",
            "li t1, 0xAAAAAAAA",
            "ori t0, t1, 0x555",
            "li t1, 0x55555555",
            "ori t0, t1, -1",
            "li t1, 0x7FFFFFFF",
            "ori t0, t1, 1",
            "li t1, 0x00000FFF",
            "ori t0, t1, 0x001",
            "li t1, 0x00000001",
            "ori t0, t1, -512",
            options(nostack, nomem)
        );
    }
    guest_bin::halt()
}
