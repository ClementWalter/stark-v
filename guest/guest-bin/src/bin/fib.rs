#![no_std]
#![no_main]

#[path = "../glue.rs"]
mod glue;

use guest_lib::{fibonacci, FibResult};

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    let n = 20;
    let result = FibResult {
        n,
        value: fibonacci(n),
    };
    glue::output(&result)
}
