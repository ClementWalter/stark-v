// First version inspired from https://github.com/openvm-org/openvm/blob/02d5a13556b167ccae0c3b817f612adf26d92fda/crates/toolchain/openvm/src/lib.rs

#![no_std]
#![cfg_attr(not(test), allow(unexpected_cfgs))]

// Compiles on the guest only
#[cfg(target_os = "zkvm")]
mod guest {
    use core::arch::{asm, global_asm};

    static STACK_TOP: u32 = 0x0020_0400;
    const CUSTOM_0_SYSTEM_OPCODE: u32 = 0x0b;
    const TERMINATE_FUNCT3: u32 = 0;
    const IMM_SHIFT: u32 = 20;
    const FUNCT3_SHIFT: u32 = 12;

    const fn encode_terminate(imm: u8) -> u32 {
        // Format: imm[11:0] | rs1=x0 | funct3 | rd=x0 | opcode
        (((imm as u32) & 0xfff) << IMM_SHIFT)
            | (TERMINATE_FUNCT3 << FUNCT3_SHIFT)
            | CUSTOM_0_SYSTEM_OPCODE
    }

    #[inline(always)]
    pub fn terminate<const EXIT_CODE: u8>() -> ! {
        unsafe {
            asm!(
                ".word {instruction}",
                instruction = const encode_terminate(EXIT_CODE),
                options(noreturn)
            )
        }
    }

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
        terminate::<1>();
    }
}

#[cfg(target_os = "zkvm")]
pub use guest::terminate;

// Macro to define the entry point for the guest
// The guest adds `stark_v::entry!(main)` to set main as the entry point.
// The macro generates a starkv_entry function wrapping the guest entry point and terminates with
// the custom-0 terminate opcode once the guest returns. Final flow: jump to _start (assembly) ->
// calls __starkv_start() -> calls starkv_entry() -> calls guest entry point -> issues terminate
#[macro_export]
macro_rules! entry {
    ($path:path) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn starkv_entry() -> ! {
            let entry_fn: fn() = $path;
            entry_fn();
            $crate::terminate::<0>();
        }
    };
}

// Trivial terminate function to satisfy compilers on non-guest targets
#[cfg(not(target_os = "zkvm"))]
#[inline(always)]
pub fn terminate<const EXIT_CODE: u8>() -> ! {
    let _ = EXIT_CODE;
    unreachable!()
}

// Trivial panic handler to satisfy compilers on non-guest targets
#[cfg(not(target_os = "zkvm"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
