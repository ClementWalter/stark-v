#![no_std]
#![no_main]

guest_lib::guest_main!(guest_lib::fib(20));
