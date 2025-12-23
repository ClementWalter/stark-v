//! Auto-generated example for muldiv.
//! Run with: cargo run --example muldiv

fn main() {
    let result = guest_lib::programs::muldiv::test_call();
    let bytes = postcard::to_allocvec(&result).unwrap();

    // Write raw bytes to stdout for piping/testing
    use std::io::Write;
    std::io::stdout().write_all(&bytes).unwrap();
}
