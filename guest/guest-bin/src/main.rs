#![no_std]
#![no_main]

mod glue;

use guest_lib::{main, ComputeResult};

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    let result = ComputeResult { value: main() };
    glue::output(&result)
}
