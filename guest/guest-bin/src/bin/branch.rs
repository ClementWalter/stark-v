#![no_std]
#![no_main]

#[path = "../glue.rs"]
mod glue;

use guest_lib::{branch_test, BranchResult};

#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    let x = 5;
    let result = BranchResult {
        x,
        value: branch_test(x),
    };
    glue::output(&result)
}
