// First version inspired from https://github.com/openvm-org/openvm/blob/02d5a13556b167ccae0c3b817f612adf26d92fda/crates/toolchain/openvm/src/lib.rs

#![no_std]
#![cfg_attr(not(test), allow(unexpected_cfgs))]

// Compiles on the guest only
#[cfg(target_os = "zkvm")]
mod guest {
    use core::arch::global_asm;

    static STACK_TOP: u32 = 0x0020_0400;

    // Set the stack pointer to the top of the stack and make it grow downwards
    // Set the entry point to __starkv_start that calls the guest entry point
    global_asm!(
        r#"
    .section .text._start;
    .globl _start;
    _start:
        .option push;
        .option norelax;
        la gp, __global_pointer$;
        .option pop;
        la sp, {stack};
        lw sp, 0(sp);
        call __starkv_start;
    "#,
        stack = sym STACK_TOP
    );

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn __starkv_start() -> ! {
        unsafe extern "C" {
            fn starkv_entry() -> !;
        }
        unsafe { starkv_entry() };
    }

    #[panic_handler]
    fn panic(_info: &core::panic::PanicInfo) -> ! {
        loop {}
    }
}

// Macro to define the entry point for the guest
// The guest adds `stark_v::entry!(main)` to set main as the entry point.
// The macro generates a starkv_entry function wrapping the guest entry point and an infinite loop.
// Final flow: jump to _start (assembly) -> calls __starkv_start() -> calls starkv_entry() -> calls guest entry point
#[macro_export]
macro_rules! entry {
    ($path:path) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn starkv_entry() -> ! {
            let entry_fn: fn() = $path;
            entry_fn();
            loop {}
        }
    };
}

// Trivial panic handler to satisfy compilers on non-guest targets
#[cfg(not(target_os = "zkvm"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unreachable!()
}
