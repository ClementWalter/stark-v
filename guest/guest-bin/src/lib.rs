#![no_std]

// =============================================================================
// Guest glue and entry macro
// =============================================================================

pub mod glue;

#[cfg(feature = "alloc")]
pub extern crate alloc;

#[cfg(target_arch = "riscv32")]
pub use guest_lib::io;
pub use guest_lib::programs;

/// Common imports for guest programs.
pub mod prelude {
    pub use crate::guest_main;
    pub use crate::halt;

    #[cfg(target_arch = "riscv32")]
    pub use crate::io;

    #[cfg(feature = "alloc")]
    pub use crate::alloc;
}

// Re-export halt for opcode test binaries
pub use glue::halt;

/// Macro to define the guest entry point with minimal boilerplate.
///
/// When the `alloc` feature is enabled, this macro initializes the heap
/// allocator before evaluating the expression, enabling use of `alloc`
/// types like `Vec`, `Box`, and `String`.
///
/// # Example
///
/// ```ignore
/// #![no_std]
/// #![no_main]
///
/// use guest_bin::prelude::*;
///
/// guest_main!(guest_lib::fib(20));
/// ```
///
/// # Example with alloc feature
///
/// ```ignore
/// #![no_std]
/// #![no_main]
///
/// use guest_bin::alloc::vec::Vec;
/// use guest_bin::prelude::*;
///
/// guest_main!({
///     let mut v = Vec::new();
///     v.push(42);
///     v.len()
/// });
/// ```
#[macro_export]
macro_rules! guest_main {
    ($expr:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn __zkvm_start() -> ! {
            // Initialize heap if alloc feature is enabled (riscv32 only)
            #[cfg(all(feature = "alloc", target_arch = "riscv32"))]
            unsafe {
                guest_lib::allocator::init_heap();
            }

            $crate::glue::output(&$expr)
        }
    };
}
