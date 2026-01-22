//! Auto-generated example for ecdsa.
//! Run with: cargo run --example ecdsa

fn main() {
    let result = guest_lib::programs::ecdsa::test_call();
    let bytes = postcard::to_allocvec(&result).unwrap();

    // Write raw bytes to stdout for piping/testing
    use std::io::Write;
    std::io::stdout().write_all(&bytes).unwrap();
}
