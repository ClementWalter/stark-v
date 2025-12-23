#![no_std]
#![no_main]

guest_bin::guest_main!(guest_lib::programs::load_merge::test_call());
