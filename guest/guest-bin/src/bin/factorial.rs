#![no_std]
#![no_main]

#[path = "../glue.rs"]
mod glue;

use guest_lib::factorial;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    glue::finalize(factorial(100))
}
