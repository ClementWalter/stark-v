//! Auto-generated example for factorial.
//! Run with: cargo run --example factorial

fn main() {
    let result = guest_lib::programs::factorial::test_call();
    let bytes = postcard::to_allocvec(&result).unwrap();

    // Write raw bytes to stdout for piping/testing
    use std::io::Write;
    std::io::stdout().write_all(&bytes).unwrap();
}
