#![no_std]
#![no_main]

guest_bin::guest_main!(guest_lib::programs::branch::test_call());
