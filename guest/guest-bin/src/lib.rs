#![no_std]

// =============================================================================
// Guest glue and entry macro
// =============================================================================

pub mod glue;

// Re-export halt for opcode test binaries
pub use glue::halt;

/// Macro to define the guest entry point with minimal boilerplate.
///
/// The expression result is serialized with postcard before writing to output.
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

/// Macro to define a guest entry point that writes raw bytes to output.
///
/// Unlike [`guest_main!`], this does not serialize with postcard — it writes
/// the raw byte slice directly. Use this when the output is already raw bytes
/// (e.g., a 32-byte hash digest).
///
/// # Example
///
/// ```ignore
/// #![no_std]
/// #![no_main]
///
/// guest_bin::guest_main_raw!({
///     let hash = compute_sha256(input);
///     hash
/// });
/// ```
#[macro_export]
macro_rules! guest_main_raw {
    ($expr:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn __zkvm_start() -> ! {
            $crate::glue::output_raw(&$expr)
        }
    };
}
