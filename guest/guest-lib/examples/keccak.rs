//! Auto-generated example for keccak.
//! Run with: cargo run --example keccak

fn main() {
    let result = guest_lib::programs::keccak::test_call();
    let bytes = postcard::to_allocvec(&result).unwrap();

    // Write raw bytes to stdout for piping/testing
    use std::io::Write;
    std::io::stdout().write_all(&bytes).unwrap();
}
