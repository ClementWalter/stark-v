//! Fibonacci with input - demonstrates using guest_lib::io for input.

#![no_std]
#![no_main]

guest_bin::guest_main!({
    // SAFETY: We are running inside the zkVM guest environment
    let n = unsafe { guest_lib::io::read_input_u32() };
    guest_lib::programs::fib::fib(n)
});
