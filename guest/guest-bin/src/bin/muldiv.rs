#![no_std]
#![no_main]

#[path = "../glue.rs"]
mod glue;

use guest_lib::{muldiv_test, MulDivResult};

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    let result = MulDivResult {
        value: muldiv_test(),
    };
    glue::output(&result)
}
