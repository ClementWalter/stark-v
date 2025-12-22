#![no_std]
#![no_main]

#[path = "../glue.rs"]
mod glue;

use guest_lib::branch_test;

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    glue::finalize(branch_test(5))
}
