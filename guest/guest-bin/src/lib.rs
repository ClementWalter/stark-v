#![no_std]

// =============================================================================
// Guest glue and entry macro
// =============================================================================

pub mod glue;

/// Macro to define the guest entry point with minimal boilerplate.
///
/// # Example
///
/// ```ignore
/// #![no_std]
/// #![no_main]
///
/// guest_bin::guest_main!(guest_lib::fib(20));
/// ```
#[macro_export]
macro_rules! guest_main {
    ($expr:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn __zkvm_start() -> ! {
            $crate::glue::output(&$expr)
        }
    };
}
