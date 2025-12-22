#![no_std]
#![no_main]

#[path = "../glue.rs"]
mod glue;

use guest_lib::{factorial, FactorialResult};

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    let n = 10;
    let result = FactorialResult {
        n,
        value: factorial(n),
    };
    glue::output(&result)
}
