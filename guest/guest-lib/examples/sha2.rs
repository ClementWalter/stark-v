//! Auto-generated example for sha2.
//! Run with: cargo run --example sha2

fn main() {
    let result = guest_lib::programs::sha2::test_call();
    let bytes = postcard::to_allocvec(&result).unwrap();

    // Write raw bytes to stdout for piping/testing
    use std::io::Write;
    std::io::stdout().write_all(&bytes).unwrap();
}
