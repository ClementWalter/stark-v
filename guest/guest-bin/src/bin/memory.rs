#![no_std]
#![no_main]

#[path = "../glue.rs"]
mod glue;

use guest_lib::{memory_test, MemoryTestResult};

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    let result = MemoryTestResult {
        sum: memory_test(),
    };
    glue::output(&result)
}
